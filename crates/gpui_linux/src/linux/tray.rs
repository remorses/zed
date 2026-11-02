//! Tray items over StatusNotifierItem, via `ksni`.
//!
//! `ksni` runs the item and its `com.canonical.dbusmenu` menu on its own
//! thread. Its callbacks only push events into the channel installed by
//! [`gpui::Platform::set_tray_event_sender`]; tray updates travel the other way
//! through [`ksni::blocking::Handle::update`].
//!
//! Whether a left click opens the menu or reports [`TrayEvent::Click`] is the
//! compile-time `ksni::Tray::MENU_ON_ACTIVATE`: it sets the item's `ItemIsMenu`
//! property and makes `Activate` fail, which is what KDE Plasma reads to open
//! the menu instead. So an item whose menu appears or disappears is registered
//! again as the other [`SniTray`] type. GNOME's AppIndicator extension calls
//! `Activate` only on a double click, so there a menu-less item reports
//! [`TrayEvent::Click`] on a double click.

use std::{
    cell::RefCell,
    path::Path,
    sync::{Arc, atomic::AtomicUsize, atomic::Ordering},
};

use futures::channel::mpsc::UnboundedSender;
use gpui::{PlatformTray, SharedString, TrayEvent, TrayMenuItem, TrayOptions};
use ksni::blocking::{Handle, TrayMethods as _};
use ksni::menu::{CheckmarkItem, MenuItem, StandardItem};
use parking_lot::Mutex;

/// The channel installed through [`gpui::Platform::set_tray_event_sender`].
pub(crate) type TrayEventSender = Arc<Mutex<Option<UnboundedSender<TrayEvent>>>>;

pub(crate) fn new_event_sender() -> TrayEventSender {
    Arc::new(Mutex::new(None))
}

/// Creates a tray item and starts its StatusNotifierItem service.
pub(crate) fn create(
    options: TrayOptions,
    events: TrayEventSender,
) -> Option<Box<dyn PlatformTray>> {
    let state = TrayState {
        id: format!("gpui-tray-{}", next_tray_id()),
        icon: options.icon.as_deref().map(icon_pixmap).unwrap_or_default(),
        title: options.title,
        tooltip: options.tooltip,
        menu: Vec::new(),
        events,
    };

    match Service::spawn(state) {
        Ok(service) => Some(Box::new(LinuxTray {
            service: RefCell::new(service),
        })),
        Err(error) => {
            log::warn!("failed to create the tray item: {error}");
            None
        }
    }
}

fn next_tray_id() -> usize {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Owns a tray item for the application.
struct LinuxTray {
    service: RefCell<Service>,
}

impl PlatformTray for LinuxTray {
    fn set_menu(&self, items: Vec<TrayMenuItem>) {
        let mut service = self.service.borrow_mut();
        if service.is_menu() != items.is_empty() {
            service.update(move |state| state.menu = items);
            return;
        }

        let Some(mut state) = service.update(|state| state.clone()) else {
            return;
        };
        state.menu = items;
        service.shutdown();
        match Service::spawn(state) {
            Ok(respawned) => *service = respawned,
            Err(error) => log::warn!("failed to register the tray item again: {error}"),
        }
    }

    fn set_tooltip(&self, tooltip: Option<SharedString>) {
        self.service
            .borrow()
            .update(move |state| state.tooltip = tooltip);
    }

    fn set_title(&self, _title: Option<SharedString>) {
        // StatusNotifierItem has no title beside the icon.
    }
}

impl Drop for LinuxTray {
    fn drop(&mut self) {
        self.service.get_mut().shutdown();
    }
}

/// The running StatusNotifierItem service, typed by whether the item is a menu.
enum Service {
    Click(Handle<SniTray<false>>),
    Menu(Handle<SniTray<true>>),
}

impl Service {
    fn spawn(state: TrayState) -> Result<Self, ksni::Error> {
        if state.menu.is_empty() {
            Ok(Self::Click(SniTray(state).spawn()?))
        } else {
            Ok(Self::Menu(SniTray(state).spawn()?))
        }
    }

    fn is_menu(&self) -> bool {
        matches!(self, Self::Menu(_))
    }

    /// Returns `None` once the service has stopped.
    fn update<R>(&self, f: impl FnOnce(&mut TrayState) -> R) -> Option<R> {
        match self {
            Self::Click(handle) => handle.update(|tray| f(&mut tray.0)),
            Self::Menu(handle) => handle.update(|tray| f(&mut tray.0)),
        }
    }

    /// Waits for the service to unregister the item.
    fn shutdown(&self) {
        match self {
            Self::Click(handle) => handle.shutdown().wait(),
            Self::Menu(handle) => handle.shutdown().wait(),
        }
    }
}

#[derive(Clone)]
struct TrayState {
    id: String,
    icon: Vec<ksni::Icon>,
    title: Option<SharedString>,
    tooltip: Option<SharedString>,
    menu: Vec<TrayMenuItem>,
    events: TrayEventSender,
}

/// `MENU_ON_ACTIVATE` is true exactly when `menu` is not empty; [`Service`]
/// keeps that true.
struct SniTray<const MENU_ON_ACTIVATE: bool>(TrayState);

impl<const MENU_ON_ACTIVATE: bool> ksni::Tray for SniTray<MENU_ON_ACTIVATE> {
    const MENU_ON_ACTIVATE: bool = MENU_ON_ACTIVATE;

    fn id(&self) -> String {
        self.0.id.clone()
    }

    fn title(&self) -> String {
        self.0.title.as_deref().unwrap_or_default().to_string()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.0.icon.clone()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: self.0.title.as_deref().unwrap_or_default().to_string(),
            description: self.0.tooltip.as_deref().unwrap_or_default().to_string(),
            ..Default::default()
        }
    }

    /// A left click. `ksni` calls this only for a menu-less item.
    fn activate(&mut self, _x: i32, _y: i32) {
        if let Some(sender) = self.0.events.lock().as_ref() {
            sender.unbounded_send(TrayEvent::Click).ok();
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        self.0
            .menu
            .iter()
            .map(|item| {
                let id = item.id.clone();
                let events = self.0.events.clone();
                let activate = Box::new(move |_tray: &mut Self| {
                    if let Some(sender) = events.lock().as_ref() {
                        sender.unbounded_send(TrayEvent::MenuItem(id.clone())).ok();
                    }
                });
                if item.checked {
                    MenuItem::Checkmark(CheckmarkItem {
                        label: item.label.to_string(),
                        enabled: item.enabled,
                        checked: true,
                        activate,
                        ..Default::default()
                    })
                } else {
                    MenuItem::Standard(StandardItem {
                        label: item.label.to_string(),
                        enabled: item.enabled,
                        activate,
                        ..Default::default()
                    })
                }
            })
            .collect()
    }
}

/// Decodes the icon into the ARGB32 pixmap the item carries: StatusNotifierItem
/// has no portable way to reference a file path.
fn icon_pixmap(path: &Path) -> Vec<ksni::Icon> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("failed to read tray icon {path:?}: {error}");
            return Vec::new();
        }
    };
    let image = match image::load_from_memory(&bytes) {
        Ok(image) => image.to_rgba8(),
        Err(error) => {
            log::warn!("failed to decode tray icon {path:?}: {error}");
            return Vec::new();
        }
    };

    let (width, height) = image.dimensions();
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        // RGBA to ARGB32 in network byte order.
        pixel.rotate_right(1);
    }
    vec![ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }]
}
