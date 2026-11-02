//! A tray item with a menu that prints every event it reports.
//!
//! Closing the window keeps the process, and the tray item, alive. Use the
//! item's Quit entry to exit; its Remove menu entry leaves an item with no
//! menu, so a click reports an event on its own.

use gpui::{
    App, Bounds, Context, QuitMode, Render, TrayEvent, TrayMenuItem, TrayOptions, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;

struct TrayExample;

impl Render for TrayExample {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_8()
            .bg(rgb(0x18181b))
            .text_color(rgb(0xf4f4f5))
            .child(div().text_2xl().child("GPUI tray item"))
            .child(
                div().text_sm().text_color(rgb(0xa1a1aa)).child(
                    "Close this window: the tray item stays. Its menu prints events and quits.",
                ),
            )
    }
}

fn menu() -> Vec<TrayMenuItem> {
    vec![
        TrayMenuItem {
            id: "say-hello".into(),
            label: "Say hello".into(),
            enabled: true,
            checked: false,
        },
        TrayMenuItem {
            id: "remove-menu".into(),
            label: "Remove menu".into(),
            enabled: true,
            checked: false,
        },
        TrayMenuItem {
            id: "quit".into(),
            label: "Quit".into(),
            enabled: true,
            checked: false,
        },
    ]
}

fn main() {
    application().run(|cx: &mut App| {
        // Without this the process would exit when the only window closes.
        cx.set_quit_mode(QuitMode::Explicit);

        let Some(tray) = cx.create_tray(TrayOptions {
            icon: None,
            title: Some("GPUI".into()),
            tooltip: Some("GPUI tray example".into()),
        }) else {
            println!("tray items are unsupported on this platform");
            return;
        };
        tray.set_menu(menu());

        // The handler owns the handle, so the item lives as long as the
        // application does.
        cx.on_tray_event(move |event, cx| match event {
            TrayEvent::Click => println!("event: click"),
            TrayEvent::MenuItem(id) if id.as_str() == "quit" => {
                println!("event: menu item `quit`");
                cx.quit();
            }
            TrayEvent::MenuItem(id) if id.as_str() == "remove-menu" => {
                println!("event: menu item `remove-menu`");
                tray.set_menu(Vec::new());
            }
            TrayEvent::MenuItem(id) => println!("event: menu item `{id}`"),
        });

        let bounds = Bounds::centered(None, size(px(560.), px(360.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Tray Example".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| TrayExample),
        )
        .expect("failed to open the tray example window");
        cx.activate(true);
        println!("tray created: close the window, then use the GPUI menu-bar item");
    });
}
