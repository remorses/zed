//! Tray items backed by `NSStatusItem` in the macOS menu bar.
//!
//! The status item is a sibling of `NSApp.mainMenu`, which the application
//! menu also builds through gpui's menu model; the two could share one model
//! later. Unlike the main menu, its actions run inside AppKit's menu-tracking
//! loop, so they only push into the event channel and never touch gpui state.

use std::{cell::RefCell, path::Path, rc::Rc};

use futures::channel::mpsc::UnboundedSender;
use gpui::{PlatformTray, SharedString, TrayEvent, TrayMenuItem, TrayOptions};
use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send, sel};
use objc2_app_kit::{
    NSControlStateValueOff, NSControlStateValueOn, NSImage, NSMenu, NSMenuItem, NSStatusBar,
    NSStatusBarButton, NSStatusItem, NSVariableStatusItemLength,
};
use objc2_foundation::{NSObject, NSSize, NSString};

/// The channel installed through [`gpui::Platform::set_tray_event_sender`].
pub(crate) type TrayEventSender = Rc<RefCell<Option<UnboundedSender<TrayEvent>>>>;

pub(crate) fn new_event_sender() -> TrayEventSender {
    Rc::new(RefCell::new(None))
}

/// Handles the events of one tray item, sending them to the application.
struct TrayActionTargetIvars {
    events: TrayEventSender,
}

define_class!(
    // SAFETY: `NSObject` has no subclassing requirements and
    // `TrayActionTarget` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[ivars = TrayActionTargetIvars]
    struct TrayActionTarget;

    impl TrayActionTarget {
        /// Sent by a menu item of the tray's menu.
        #[unsafe(method(trayMenuItemSelected:))]
        fn menu_item_selected(&self, sender: &NSMenuItem) {
            let Some(id) = sender
                .representedObject()
                .and_then(|object| object.downcast::<NSString>().ok())
            else {
                return;
            };
            send(
                &self.ivars().events,
                TrayEvent::MenuItem(SharedString::from(id.to_string())),
            );
        }

        /// Sent by the tray item's button, which only happens without a menu.
        #[unsafe(method(trayButtonClicked:))]
        fn button_clicked(&self, _sender: &NSStatusBarButton) {
            send(&self.ivars().events, TrayEvent::Click);
        }
    }
);

impl TrayActionTarget {
    fn new(events: TrayEventSender) -> Retained<Self> {
        let this = Self::alloc().set_ivars(TrayActionTargetIvars { events });
        // SAFETY: `NSObject`'s `init` is its designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// The only thing a native action may do: hand the event to the channel.
fn send(events: &TrayEventSender, event: TrayEvent) {
    if let Some(sender) = events.borrow().as_ref() {
        sender.unbounded_send(event).ok();
    }
}

/// A tray item in the macOS menu bar.
pub(crate) struct MacTray {
    status_item: Retained<NSStatusItem>,
    button: Retained<NSStatusBarButton>,
    /// Menu item targets are weak, so the target is kept alive here.
    target: Retained<TrayActionTarget>,
}

impl MacTray {
    pub(crate) fn new(options: TrayOptions, events: TrayEventSender) -> Option<Self> {
        let mtm = MainThreadMarker::new()?;
        let status_item =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
        let Some(button) = status_item.button(mtm) else {
            NSStatusBar::systemStatusBar().removeStatusItem(&status_item);
            return None;
        };
        let target = TrayActionTarget::new(events);

        match options.icon.as_deref().and_then(load_image) {
            Some(image) => button.setImage(Some(&image)),
            None => {
                if let Some(title) = options.title {
                    button.setTitle(&NSString::from_str(&title));
                }
            }
        }
        button.setToolTip(
            options
                .tooltip
                .as_deref()
                .map(NSString::from_str)
                .as_deref(),
        );

        // SAFETY: `TrayActionTarget` implements `trayButtonClicked:`, the
        // signature AppKit sends for a control action.
        unsafe {
            button.setTarget(Some(&target));
            button.setAction(Some(sel!(trayButtonClicked:)));
        }

        Some(Self {
            status_item,
            button,
            target,
        })
    }
}

impl PlatformTray for MacTray {
    fn set_menu(&self, items: Vec<TrayMenuItem>) {
        let Some(mtm) = MainThreadMarker::new() else {
            log::warn!("tray menus must be updated on the main thread");
            return;
        };

        let menu = (!items.is_empty()).then(|| {
            let menu = NSMenu::new(mtm);
            // Without this AppKit would enable items whenever their target
            // responds to the action, ignoring `TrayMenuItem::enabled`.
            menu.setAutoenablesItems(false);
            for item in items {
                menu.addItem(&menu_item(&item, &self.target, mtm));
            }
            menu
        });
        self.status_item.setMenu(menu.as_deref());
    }

    fn set_tooltip(&self, tooltip: Option<SharedString>) {
        if MainThreadMarker::new().is_none() {
            log::warn!("tray tooltips must be updated on the main thread");
            return;
        }
        self.button
            .setToolTip(tooltip.as_deref().map(NSString::from_str).as_deref());
    }

    fn set_title(&self, title: Option<SharedString>) {
        if MainThreadMarker::new().is_none() {
            log::warn!("tray titles must be updated on the main thread");
            return;
        }
        self.button
            .setTitle(&NSString::from_str(title.as_deref().unwrap_or_default()));
    }
}

impl Drop for MacTray {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.status_item);
    }
}

fn menu_item(
    item: &TrayMenuItem,
    target: &TrayActionTarget,
    mtm: MainThreadMarker,
) -> Retained<NSMenuItem> {
    let menu_item = NSMenuItem::new(mtm);
    menu_item.setTitle(&NSString::from_str(&item.label));
    menu_item.setEnabled(item.enabled);
    menu_item.setState(if item.checked {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });

    // SAFETY: the target outlives the menu item, and `TrayActionTarget`
    // implements the action's selector.
    unsafe {
        menu_item.setRepresentedObject(Some(&NSString::from_str(&item.id)));
        menu_item.setTarget(Some(target));
        menu_item.setAction(Some(sel!(trayMenuItemSelected:)));
    }
    menu_item
}

/// The height AppKit's own status items draw their images at, in points.
const MENU_BAR_ICON_HEIGHT: f64 = 18.0;

/// Loads an image from disk, fitted to the menu bar's height.
///
/// A status item draws its image at the image's own point size, so an app
/// icon would otherwise fill the bar. Template rendering is left to the
/// artwork: the image is used as-is so that colored icons keep their color.
fn load_image(path: &Path) -> Option<Retained<NSImage>> {
    let path = path.to_str()?;
    let image = NSImage::initWithContentsOfFile(NSImage::alloc(), &NSString::from_str(path))?;
    let size = image.size();
    if size.height > 0.0 {
        let scale = MENU_BAR_ICON_HEIGHT / size.height;
        image.setSize(NSSize::new(size.width * scale, MENU_BAR_ICON_HEIGHT));
    }
    Some(image)
}
