use gtk4::gdk;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::ui::skin_overlay::SkinOverlayController;
use std::cell::Cell;
use std::rc::Rc;

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
}
