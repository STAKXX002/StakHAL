use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use stakhal_ui::ui::skin_overlay::SkinOverlayController;
use stakhal_ui::ui::tokens;
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[test]
fn test_skin_overlay_widgets_are_passthrough() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping skin overlay widget test: {err}");
        return;
    }
    if gdk::Display::default().is_none() {
        eprintln!("GDK default display not available, skipping skin overlay widget test");
        return;
    }
    let _ = adw::init();

    let window = adw::ApplicationWindow::builder().build();
    let skin_overlay = gtk4::Overlay::new();
    let header_overlay = gtk4::Overlay::new();
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    skin_overlay.set_child(Some(&content));
    window.set_content(Some(&skin_overlay));

    let swapped = Rc::new(Cell::new(false));
    let swapped_cb = Rc::clone(&swapped);

    let ctrl = SkinOverlayController::setup(
        &window,
        &skin_overlay,
        &header_overlay,
        Rc::new(move || {
            swapped_cb.set(true);
        }),
    );

    assert!(!ctrl.ambient_area().can_target());
    assert!(!ctrl.ambient_area().can_focus());
    assert!(!ctrl.shimmer_area().can_target());
    assert!(!ctrl.shimmer_area().can_focus());

    // Reduced motion triggers instant swap without starting loops
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    ctrl.trigger_unlock();
    assert!(swapped.get());
    assert!(!ctrl.is_effects_active());

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    // Exercise live ripple animation and cleanup if host animations are enabled
    if tokens::motion::is_animations_enabled() {
        let win2 = adw::ApplicationWindow::builder()
            .default_width(320)
            .default_height(240)
            .build();
        let ov2 = gtk4::Overlay::new();
        let hov2 = gtk4::Overlay::new();
        let box2 = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        ov2.set_child(Some(&box2));
        win2.set_content(Some(&ov2));
        win2.present();
        while glib::MainContext::default().iteration(false) {}

        let swapped2 = Rc::new(Cell::new(false));
        let swapped2_cb = Rc::clone(&swapped2);
        let ctrl2 = SkinOverlayController::setup(
            &win2,
            &ov2,
            &hov2,
            Rc::new(move || {
                swapped2_cb.set(true);
            }),
        );

        ctrl2.trigger_unlock();

        // Newly added ripple overlay sits as last_child of ov2 and must be pass-through
        let ripple_widget = ov2.last_child().expect("ripple overlay added");
        assert!(!ripple_widget.can_target());
        assert!(!ripple_widget.can_focus());
        assert!(
            !swapped2.get(),
            "Skin must not swap at t=0ms before 50% progress"
        );

        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(600) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            swapped2.get(),
            "Skin swap must have occurred after 50% (450ms) progress"
        );

        while start.elapsed() < Duration::from_millis(1050) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        while glib::MainContext::default().iteration(false) {}

        // After 900ms, ripple overlay must have been removed leaving only content + ambient_area
        assert!(
            ripple_widget.parent().is_none(),
            "Ripple DrawingArea must be removed from overlay by 900ms"
        );
        win2.close();
    }

    window.close();
}
