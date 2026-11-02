//! Tray items backed by `Shell_NotifyIconW` and a hidden message window.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    ffi::OsStr,
    mem::size_of,
    os::windows::ffi::OsStrExt as _,
    path::Path,
    rc::Rc,
    sync::OnceLock,
};

use futures::channel::mpsc::UnboundedSender;
use gpui::{PlatformTray, SharedString, TrayEvent, TrayMenuItem, TrayOptions};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
                Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, ChangeWindowMessageFilterEx, CreatePopupMenu, CreateWindowExW,
                DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, GetCursorPos,
                GetSystemMetrics, HICON, IMAGE_ICON, LR_LOADFROMFILE, LoadImageW, MF_CHECKED,
                MF_GRAYED, MF_STRING, MSGFLT_ALLOW, PostMessageW, RegisterClassW,
                RegisterWindowMessageW, SM_CXSMICON, SM_CYSMICON, SetForegroundWindow,
                TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_APP, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
            },
        },
    },
    core::{PCWSTR, w},
};

/// The message the notification area posts to the tray's window.
const TRAY_CALLBACK_MESSAGE: u32 = WM_APP + 1;

/// Explorer broadcasts this to top-level windows when it creates the taskbar
/// again, after a restart or crash, by which point every item is gone.
/// `0` means registering it failed.
fn taskbar_created_message() -> u32 {
    static MESSAGE: OnceLock<u32> = OnceLock::new();
    // SAFETY: the name is a static null-terminated string.
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

/// The channel installed through [`gpui::Platform::set_tray_event_sender`].
pub(crate) type TrayEventSender = Rc<RefCell<Option<UnboundedSender<TrayEvent>>>>;

pub(crate) fn new_event_sender() -> TrayEventSender {
    Rc::new(RefCell::new(None))
}

thread_local! {
    /// The live trays, keyed by their message window, so the window procedure
    /// can find the tray that owns a message.
    static TRAYS: RefCell<HashMap<isize, Rc<WindowsTray>>> = RefCell::new(HashMap::new());
    static CLASS_REGISTERED: Cell<bool> = const { Cell::new(false) };
}

/// A tray item in the notification area.
pub(crate) struct WindowsTray {
    window: HWND,
    icon: Option<HICON>,
    /// A copy is passed to each `Shell_NotifyIconW` call, which can dispatch
    /// sent messages, `TaskbarCreated` included, back into this tray.
    notification: Cell<NOTIFYICONDATAW>,
    menu: RefCell<Vec<TrayMenuItem>>,
    events: TrayEventSender,
}

impl WindowsTray {
    pub(crate) fn new(options: TrayOptions, events: TrayEventSender) -> Option<Rc<Self>> {
        let window = unsafe { create_message_window() }?;
        // An elevated process would otherwise not receive the lower-integrity
        // Explorer's broadcast.
        // SAFETY: `window` was just created on this thread.
        unsafe {
            let _ =
                ChangeWindowMessageFilterEx(window, taskbar_created_message(), MSGFLT_ALLOW, None);
        }
        let icon = options.icon.as_deref().and_then(load_icon);
        // A notification-area item cannot show text, so without an icon of its
        // own it borrows the executable's (shared, so never destroyed).
        let shown_icon = icon.or_else(|| crate::platform::load_icon().ok());
        let tooltip = options.tooltip.or(options.title);

        let mut notification = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: window,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_CALLBACK_MESSAGE,
            hIcon: shown_icon.unwrap_or_default(),
            ..Default::default()
        };
        if let Some(tooltip) = &tooltip {
            write_wide(&mut notification.szTip, tooltip);
        }
        if shown_icon.is_some() {
            notification.uFlags |= NIF_ICON;
        }

        // SAFETY: `notification` is fully initialized and outlives the call.
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &notification) }.as_bool() {
            log::warn!("failed to add the tray item to the notification area");
            unsafe {
                let _ = DestroyWindow(window);
                if let Some(icon) = icon {
                    let _ = DestroyIcon(icon);
                }
            }
            return None;
        }

        let tray = Rc::new(Self {
            window,
            icon,
            notification: Cell::new(notification),
            menu: RefCell::new(Vec::new()),
            events,
        });
        TRAYS.with(|trays| {
            trays.borrow_mut().insert(window.0 as isize, tray.clone());
        });
        Some(tray)
    }

    fn set_menu(&self, items: Vec<TrayMenuItem>) {
        *self.menu.borrow_mut() = items;
    }

    fn set_title(&self, _title: Option<SharedString>) {
        // Notification-area items have no text beside the icon.
    }

    fn set_tooltip(&self, tooltip: Option<SharedString>) {
        let mut notification = self.notification.get();
        notification.szTip.fill(0);
        if let Some(tooltip) = &tooltip {
            write_wide(&mut notification.szTip, tooltip);
        }
        notification.uFlags |= NIF_TIP;
        self.notification.set(notification);
        // SAFETY: the notification data still describes this item.
        unsafe {
            if !Shell_NotifyIconW(NIM_MODIFY, &notification).as_bool() {
                log::warn!("failed to update the tray item's tooltip");
            }
        }
    }

    /// Adds the item again once Explorer has created a new taskbar.
    fn restore(&self) {
        // SAFETY: the notification data still describes this item.
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &self.notification.get()) }.as_bool() {
            log::warn!("failed to add the tray item again after the taskbar restarted");
        }
    }

    /// Opens the popup menu at the cursor and reports the chosen item.
    fn show_menu(&self) {
        if self.menu.borrow().is_empty() {
            return;
        }

        // SAFETY: every handle used here is owned by this tray for the duration
        // of the call, and the menu is built and destroyed within it.
        unsafe {
            let Ok(menu) = CreatePopupMenu() else {
                log::warn!("failed to create the tray item's menu");
                return;
            };
            let ids = {
                let items = self.menu.borrow();
                let mut ids = Vec::with_capacity(items.len());
                for (index, item) in items.iter().enumerate() {
                    let mut flags = MF_STRING;
                    if !item.enabled {
                        flags |= MF_GRAYED;
                    }
                    if item.checked {
                        flags |= MF_CHECKED;
                    }
                    let label = wide(&item.label);
                    if let Err(error) = AppendMenuW(menu, flags, index + 1, PCWSTR(label.as_ptr()))
                    {
                        log::warn!("failed to add a tray menu item: {error}");
                    }
                    ids.push(item.id.clone());
                }
                ids
            };

            let mut position = POINT::default();
            let _ = GetCursorPos(&mut position);
            // Without a foreground owner the menu stays open when the user
            // clicks elsewhere.
            let _ = SetForegroundWindow(self.window);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                position.x,
                position.y,
                None,
                self.window,
                None,
            )
            .0 as usize;
            let _ = PostMessageW(Some(self.window), WM_NULL, WPARAM(0), LPARAM(0));
            let _ = DestroyMenu(menu);

            if let Some(id) = command.checked_sub(1).and_then(|index| ids.get(index)) {
                send(&self.events, TrayEvent::MenuItem(id.clone()));
            }
        }
    }

    fn remove(&self) {
        TRAYS.with(|trays| {
            trays.borrow_mut().remove(&(self.window.0 as isize));
        });
        // SAFETY: the tray's handles are released exactly once, here.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.notification.get());
            let _ = DestroyWindow(self.window);
            if let Some(icon) = self.icon {
                let _ = DestroyIcon(icon);
            }
        }
    }
}

/// Owns a tray item for the application.
pub(crate) struct WindowsTrayHandle {
    tray: Rc<WindowsTray>,
}

impl WindowsTrayHandle {
    pub(crate) fn new(tray: Rc<WindowsTray>) -> Self {
        Self { tray }
    }
}

impl PlatformTray for WindowsTrayHandle {
    fn set_menu(&self, items: Vec<TrayMenuItem>) {
        self.tray.set_menu(items);
    }

    fn set_tooltip(&self, tooltip: Option<SharedString>) {
        self.tray.set_tooltip(tooltip);
    }

    fn set_title(&self, title: Option<SharedString>) {
        self.tray.set_title(title);
    }
}

impl Drop for WindowsTrayHandle {
    fn drop(&mut self) {
        self.tray.remove();
    }
}

/// The only thing a native action may do: hand the event to the channel.
fn send(events: &TrayEventSender, event: TrayEvent) {
    if let Some(sender) = events.borrow().as_ref() {
        sender.unbounded_send(event).ok();
    }
}

fn tray_for(window: HWND) -> Option<Rc<WindowsTray>> {
    TRAYS.with(|trays| trays.borrow().get(&(window.0 as isize)).cloned())
}

/// Creates the hidden window that receives the notification area's messages.
///
/// The window stays invisible but is a regular top-level window, which
/// `SetForegroundWindow` requires for the popup menu to close correctly.
unsafe fn create_message_window() -> Option<HWND> {
    let instance = unsafe { GetModuleHandleW(None) }.ok()?;
    let instance = HINSTANCE(instance.0);

    CLASS_REGISTERED.with(|registered| {
        if !registered.get() {
            let class = WNDCLASSW {
                lpfnWndProc: Some(tray_window_proc),
                hInstance: instance,
                lpszClassName: w!("GPUI_Tray_Window"),
                ..Default::default()
            };
            // SAFETY: the class is registered once per UI thread.
            if unsafe { RegisterClassW(&class) } == 0 {
                log::warn!("failed to register the tray window class");
                return None;
            }
            registered.set(true);
        }

        // SAFETY: the class is registered above; the window is created on the
        // thread that dispatches its messages.
        unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("GPUI_Tray_Window"),
                w!("GPUI tray"),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .ok()
    })
}

/// # Safety
///
/// `window` must be a window created by [`create_message_window`].
unsafe extern "system" fn tray_window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let taskbar_created = taskbar_created_message();
    if taskbar_created != 0 && message == taskbar_created {
        if let Some(tray) = tray_for(window) {
            tray.restore();
        }
        return LRESULT(0);
    }
    if message != TRAY_CALLBACK_MESSAGE {
        // SAFETY: forwarding an unhandled message is what the default
        // procedure is for.
        return unsafe { DefWindowProcW(window, message, wparam, lparam) };
    }

    if let Some(tray) = tray_for(window) {
        match lparam.0 as u32 {
            WM_LBUTTONUP if tray.menu.borrow().is_empty() => {
                send(&tray.events, TrayEvent::Click);
            }
            WM_LBUTTONUP | WM_RBUTTONUP => tray.show_menu(),
            _ => {}
        }
    }
    LRESULT(0)
}

fn load_icon(path: &Path) -> Option<HICON> {
    let path = wide_os(path.as_os_str());
    // SAFETY: `path` is null-terminated and only read during the call.
    unsafe {
        let size = GetSystemMetrics(SM_CXSMICON);
        LoadImageW(
            None,
            PCWSTR(path.as_ptr()),
            IMAGE_ICON,
            size,
            GetSystemMetrics(SM_CYSMICON),
            LR_LOADFROMFILE,
        )
        .ok()
        .map(|handle| HICON(handle.0))
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn wide_os(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain([0]).collect()
}

/// Writes `text` into a fixed-size UTF-16 field, truncating to fit the
/// terminator.
fn write_wide(field: &mut [u16], text: &str) {
    field.fill(0);
    let limit = field.len().saturating_sub(1);
    for (slot, unit) in field.iter_mut().zip(text.encode_utf16()).take(limit) {
        *slot = unit;
    }
}
