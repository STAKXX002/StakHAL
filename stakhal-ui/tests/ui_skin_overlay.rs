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
fn test_skin_overlay_widgets_and_thin_shimmer_strip() {
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
    let header_bar = adw::HeaderBar::new();
    header_overlay.set_child(Some(&header_bar));
    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    content.append(&header_overlay);
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

    assert!(!ctrl.shimmer_area().can_target());
    assert!(!ctrl.shimmer_area().can_focus());
    assert!(
        ctrl.shimmer_area().height_request() <= 3,
        "Shimmer overlay must have height_request <= 3, got {}",
        ctrl.shimmer_area().height_request()
    );
    assert_eq!(
        ctrl.shimmer_area().valign(),
        gtk4::Align::Start,
        "Shimmer overlay must use Align::Start"
    );
    assert!(
        !ctrl.shimmer_area().vexpands(),
        "Shimmer overlay must not vertically expand"
    );

    // Reduced motion triggers instant swap without starting shimmer or ripple
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    ctrl.trigger_unlock();
    assert!(swapped.get());
    assert!(!ctrl.is_effects_active());
    assert!(!ctrl.is_shimmer_running());

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    // Exercise live ripple animation, cleanup, strip geometry, and inactive pause if animations enabled
    if tokens::motion::is_animations_enabled() {
        let win2 = adw::ApplicationWindow::builder()
            .default_width(400)
            .default_height(300)
            .build();
        let ov2 = gtk4::Overlay::new();
        let hov2 = gtk4::Overlay::new();
        let hdr2 = adw::HeaderBar::new();
        hov2.set_child(Some(&hdr2));
        let box2 = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        box2.append(&hov2);
        let body2 = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        body2.set_vexpand(true);
        box2.append(&body2);
        ov2.set_child(Some(&box2));
        win2.set_content(Some(&ov2));
        win2.present();

        let layout_start = Instant::now();
        while (hdr2.height() <= 0 || hov2.height() <= 0)
            && layout_start.elapsed() < Duration::from_secs(1)
        {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }

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

        let alloc_start = Instant::now();
        while ctrl2.shimmer_area().height() == 0 && alloc_start.elapsed() < Duration::from_secs(1) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        let bounds = ctrl2
            .shimmer_area()
            .compute_bounds(&hov2)
            .expect("shimmer_area bounds inside header_overlay");
        assert!(
            bounds.height() > 0.0 && bounds.height() <= 3.0,
            "Allocated shimmer strip height {} must be <= 3",
            bounds.height()
        );
        assert!(
            bounds.width() >= (hdr2.width() as f32 - 2.0) && bounds.width() > 300.0,
            "Allocated shimmer strip width {} must match header width {}",
            bounds.width(),
            hdr2.width()
        );
        let expected_y = (hdr2.height() as f32 - bounds.height()).max(0.0);
        assert!(
            (bounds.y() - expected_y).abs() <= 1.0,
            "Shimmer strip y={} must sit at header bottom edge (expected {})",
            bounds.y(),
            expected_y
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
        assert!(ctrl2.is_effects_active());

        while start.elapsed() < Duration::from_millis(1050) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        while glib::MainContext::default().iteration(false) {}

        // After 900ms, ripple overlay must have been removed leaving only content_box
        assert!(
            ripple_widget.parent().is_none(),
            "Ripple DrawingArea must be removed from overlay by 900ms"
        );

        // Hiding the window must stop the shimmer tick callback cleanly (ControlFlow::Break)
        win2.set_visible(false);
        for _ in 0..10 {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !ctrl2.is_shimmer_running(),
            "Shimmer tick callback must stop when window is hidden/inactive"
        );

        // Re-showing activates/resumes cleanly if compositor makes it active
        win2.present();
        let act_start = Instant::now();
        while !win2.is_active() && act_start.elapsed() < Duration::from_millis(300) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        if win2.is_active() {
            while glib::MainContext::default().iteration(false) {}
            assert!(
                ctrl2.is_shimmer_running(),
                "Shimmer tick callback must resume when window becomes active and visible"
            );
        }

        win2.close();
    }

    window.close();
}
