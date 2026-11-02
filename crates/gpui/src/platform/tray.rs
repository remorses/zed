//! Tray items: the small status items an operating system shows outside of the
//! application's windows.
//!
//! A tray item lives in the macOS menu bar, the Windows notification area, or
//! Linux's StatusNotifier host. It is the sibling of the application menu bar,
//! not part of it.
//!
//! Events never reach the application from a native callback: backends push
//! them into the channel installed by [`Platform::set_tray_event_sender`], and
//! a foreground task drains it. Native menu-tracking loops leave the [`App`]
//! in whatever state the interrupted turn had, so they must not touch it.
//!
//! Tray items outlive the application's windows, so an app that shows one
//! usually wants [`QuitMode::Explicit`](crate::QuitMode::Explicit) as well.

use crate::SharedString;
use std::path::PathBuf;

/// The appearance of a new tray item.
///
/// Set at least one of [`Self::icon`] and [`Self::title`]: an item with neither
/// takes up space in the status area without showing anything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrayOptions {
    /// Path to an image shown in the tray.
    ///
    /// The formats are the platform's: `.icns` and `.png` on macOS, `.ico` on
    /// Windows, and anything the `image` crate decodes on Linux. When the image
    /// cannot be loaded, or this is `None`, the item falls back to
    /// [`Self::title`].
    pub icon: Option<PathBuf>,

    /// Text shown when there is no icon.
    pub title: Option<SharedString>,

    /// Text shown while the pointer hovers over the item.
    pub tooltip: Option<SharedString>,
}

/// One entry of a tray item's menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayMenuItem {
    /// Identifies the item in [`TrayEvent::MenuItem`].
    pub id: SharedString,

    /// The text shown for the item.
    pub label: SharedString,

    /// Whether choosing the item reports an event.
    pub enabled: bool,

    /// Whether the item is drawn with a checkmark.
    pub checked: bool,
}

/// An event reported by a tray item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    /// The item was clicked while it had no menu.
    ///
    /// Clicks open the menu when the item has one, so this is only reported
    /// after [`TrayHandle::set_menu`] was given an empty list.
    Click,

    /// The menu item with this id was chosen.
    MenuItem(SharedString),
}

/// A tray item as provided by a platform backend.
///
/// Dropping it removes the item from the system.
pub trait PlatformTray: 'static {
    /// Replaces the item's menu. An empty list removes the menu, so later
    /// clicks report [`TrayEvent::Click`].
    fn set_menu(&self, items: Vec<TrayMenuItem>);

    /// Replaces the text shown while the pointer hovers over the item.
    ///
    /// Not every platform shows hover text; implement it where it exists.
    fn set_tooltip(&self, tooltip: Option<SharedString>);

    /// Replaces the text shown beside the item's icon.
    ///
    /// Only macOS draws a title next to the icon; other platforms ignore it.
    fn set_title(&self, title: Option<SharedString>);
}

/// A tray item that is currently in the system's status area.
///
/// Dropping the handle removes the item.
#[must_use = "the item is removed from the status area when the handle is dropped"]
pub struct TrayHandle {
    inner: Box<dyn PlatformTray>,
}

impl TrayHandle {
    pub(crate) fn new(inner: Box<dyn PlatformTray>) -> Self {
        Self { inner }
    }

    /// Replaces the item's menu.
    ///
    /// Pass an empty list to remove the menu, so clicks report
    /// [`TrayEvent::Click`] again.
    pub fn set_menu(&self, items: Vec<TrayMenuItem>) {
        self.inner.set_menu(items);
    }

    /// Replaces the text shown while the pointer hovers over the item.
    ///
    /// macOS shows it as the status item's hover text, Windows as the
    /// notification-area tooltip (the only label a Windows item has), and
    /// Linux as the StatusNotifierItem's tooltip description.
    pub fn set_tooltip(&self, tooltip: Option<SharedString>) {
        self.inner.set_tooltip(tooltip);
    }

    /// Replaces the text shown beside the item's icon.
    ///
    /// Only macOS draws a title next to the icon; this is a no-op on Windows
    /// and Linux, which have no equivalent.
    pub fn set_title(&self, title: Option<SharedString>) {
        self.inner.set_title(title);
    }

    /// Removes the item from the system.
    ///
    /// Equivalent to dropping the handle.
    pub fn remove(self) {
        drop(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestAppContext;
    use futures::channel::mpsc::UnboundedSender;
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default)]
    struct Calls {
        menu: Option<Vec<TrayMenuItem>>,
        tooltip: Option<SharedString>,
        title: Option<SharedString>,
        removed: bool,
    }

    struct FakeTray(Rc<RefCell<Calls>>);

    impl PlatformTray for FakeTray {
        fn set_menu(&self, items: Vec<TrayMenuItem>) {
            self.0.borrow_mut().menu = Some(items);
        }

        fn set_tooltip(&self, tooltip: Option<SharedString>) {
            self.0.borrow_mut().tooltip = tooltip;
        }

        fn set_title(&self, title: Option<SharedString>) {
            self.0.borrow_mut().title = title;
        }
    }

    impl Drop for FakeTray {
        fn drop(&mut self) {
            self.0.borrow_mut().removed = true;
        }
    }

    #[test]
    fn handle_forwards_changes_and_removal() {
        let calls = Rc::new(RefCell::new(Calls::default()));
        let handle = TrayHandle::new(Box::new(FakeTray(calls.clone())));

        let item = TrayMenuItem {
            id: "open".into(),
            label: "Open".into(),
            enabled: true,
            checked: false,
        };
        handle.set_menu(vec![item.clone()]);
        handle.set_tooltip(Some("gpui".into()));
        handle.set_title(Some("GPUI".into()));

        assert_eq!(calls.borrow().menu.as_deref(), Some(&[item][..]));
        assert_eq!(calls.borrow().tooltip.as_deref(), Some("gpui"));
        assert_eq!(calls.borrow().title.as_deref(), Some("GPUI"));
        assert!(!calls.borrow().removed);

        handle.remove();
        assert!(calls.borrow().removed);
    }

    fn sender(cx: &TestAppContext) -> UnboundedSender<TrayEvent> {
        cx.tray_event_sender()
            .expect("registering a handler installs a sender")
    }

    /// Tray events come from native callbacks that must not touch the app
    /// (AppKit's menu tracking interrupts a turn), so the sender may only fill
    /// the channel: the handler runs when the foreground drains it.
    #[gpui::test]
    fn platform_sender_defers_the_handler_to_the_foreground(cx: &mut TestAppContext) {
        let calls = Rc::new(RefCell::new(Vec::new()));
        cx.update(|cx| {
            let calls = calls.clone();
            cx.on_tray_event(move |event, _| calls.borrow_mut().push(event));
        });

        let sender = sender(cx);
        std::thread::spawn(move || sender.unbounded_send(TrayEvent::Click))
            .join()
            .expect("sending thread")
            .expect("channel open");
        assert!(
            calls.borrow().is_empty(),
            "the sender must not run the handler"
        );

        cx.run_until_parked();
        assert_eq!(calls.borrow().as_slice(), &[TrayEvent::Click]);
    }

    /// Registering again must leave only the new handler connected; the old
    /// drain is canceled with its task.
    #[gpui::test]
    fn replacing_the_handler_keeps_only_the_new_handler(cx: &mut TestAppContext) {
        let previous = Rc::new(RefCell::new(Vec::new()));
        let current = Rc::new(RefCell::new(Vec::new()));
        cx.update(|cx| {
            let previous = previous.clone();
            cx.on_tray_event(move |event, _| previous.borrow_mut().push(event));
            let current = current.clone();
            cx.on_tray_event(move |event, _| current.borrow_mut().push(event));
        });

        sender(cx)
            .unbounded_send(TrayEvent::MenuItem("quit".into()))
            .expect("channel open");
        cx.run_until_parked();

        assert!(previous.borrow().is_empty());
        assert_eq!(
            current.borrow().as_slice(),
            &[TrayEvent::MenuItem("quit".into())]
        );
    }
}
