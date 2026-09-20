use gtk4::gdk;
use gtk4::gio;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::build_ui;

#[test]
fn test_ui_build_smoke() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping UI smoke test: {}", err);
        return;
    }
    if gdk::Display::default().is_none() {
        eprintln!("GDK default display not available, skipping UI smoke test");
        return;
    }
    let _ = adw::init();

    let app = adw::Application::builder()
        .application_id("com.stakhal.ui.smoke_test")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    if let Err(err) = app.register(gio::Cancellable::NONE) {
        eprintln!("Failed to register GTK application in test: {}", err);
        return;
    }

    app.connect_activate(build_ui);
    app.activate();

    let windows = app.windows();
    assert!(
        !windows.is_empty(),
        "Expected ApplicationWindow to be constructed during build_ui"
    );

    fn find_button(widget: &gtk4::Widget, label: &str) -> Option<gtk4::Button> {
        if let Ok(btn) = widget.clone().downcast::<gtk4::Button>() {
            if btn.label().as_deref() == Some(label) {
                return Some(btn);
            }
        }
        let mut child = widget.first_child();
        while let Some(c) = child {
            if let Some(found) = find_button(&c, label) {
                return Some(found);
            }
            child = c.next_sibling();
        }
        None
    }

    if let Some(win) = windows.first() {
        if let Some(btn_load) = find_button(win.upcast_ref(), "Load Project") {
            if btn_load.is_sensitive() {
                btn_load.emit_clicked();
            }
        }
    }
}
