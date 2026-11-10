use gtk4::gdk;
use gtk4::glib;
use gtk4::graphene;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use stakhal_ui::apply_skin;
use stakhal_ui::ui;
use stakhal_ui::ui::skin_overlay::{
    append_sparkle_nodes, capture_window_texture, snapshot_reveal_nodes, RevealOverlay,
    SkinOverlayController, Sparkle,
};
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
    let Some(display) = gdk::Display::default() else {
        eprintln!("GDK default display not available, skipping skin overlay widget test");
        return;
    };
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

    // Reduced motion triggers instant swap without capture, reveal overlay, or shimmer
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    ctrl.trigger_unlock();
    assert!(swapped.get());
    assert_eq!(
        skin_overlay.last_child().as_ref(),
        Some(content.upcast_ref::<gtk4::Widget>()),
        "Reduced motion must not add a reveal overlay"
    );
    assert!(!ctrl.is_effects_active());
    assert!(!ctrl.is_shimmer_running());

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    // Fallback on unrealized/unpresented window: capture returns None, unlock swaps instantly without panicking
    if tokens::motion::is_animations_enabled() {
        let unpresented = adw::ApplicationWindow::builder().build();
        let u_ov = gtk4::Overlay::new();
        let u_hov = gtk4::Overlay::new();
        let u_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        u_ov.set_child(Some(&u_box));
        unpresented.set_content(Some(&u_ov));
        assert!(
            capture_window_texture(&unpresented, &u_ov).is_none(),
            "Capture on unpresented window must fail cleanly with None"
        );
        let u_swapped = Rc::new(Cell::new(false));
        let u_swapped_cb = Rc::clone(&u_swapped);
        let u_ctrl = SkinOverlayController::setup(
            &unpresented,
            &u_ov,
            &u_hov,
            Rc::new(move || {
                u_swapped_cb.set(true);
            }),
        );
        u_ctrl.trigger_unlock();
        assert!(
            u_swapped.get(),
            "Capture failure must still trigger immediate fallback skin swap"
        );
        assert_eq!(
            u_ov.last_child().as_ref(),
            Some(u_box.upcast_ref::<gtk4::Widget>()),
            "Fallback skin swap must not attach a reveal overlay"
        );
        unpresented.close();
    }

    // Exercise live circular reveal capture, immediate underlying skin swap, cleanup,
    // strip geometry, and inactive pause if animations are enabled.
    if tokens::motion::is_animations_enabled() {
        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
        let provider = gtk4::CssProvider::new();
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        apply_skin(&provider, tokens::Palette::DEFAULT);

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
        let body2 = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        body2.set_vexpand(true);
        let btn_shine = gtk4::Button::builder()
            .label("Build & Flash")
            .css_classes(vec![
                "stakhal-btn".to_string(),
                "suggested-action".to_string(),
            ])
            .build();
        let card2 = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(vec!["card".to_string()])
            .build();
        let list2 = gtk4::ListBox::new();
        let row_a = ui::main_panel::create_peripheral_row("USART2", Some("Async"), 4);
        let row_b = ui::main_panel::create_peripheral_row("SPI1", Some("Master"), 3);
        let row_c = ui::main_panel::create_peripheral_row("TIM2", Some("PWM"), 2);
        let row_ephemeral = ui::main_panel::create_peripheral_row("I2C1", Some("Fast"), 2);
        list2.append(&row_a);
        list2.append(&row_b);
        list2.append(&row_c);
        list2.append(&row_ephemeral);
        card2.append(&list2);
        body2.append(&btn_shine);
        body2.append(&card2);
        box2.append(&body2);
        ov2.set_child(Some(&box2));
        win2.set_content(Some(&ov2));
        win2.present();

        let swapped2 = Rc::new(Cell::new(false));
        let swapped2_cb = Rc::clone(&swapped2);
        let provider_swap = provider.clone();
        let ctrl2 = SkinOverlayController::setup(
            &win2,
            &ov2,
            &hov2,
            Rc::new(move || {
                swapped2_cb.set(true);
                apply_skin(&provider_swap, tokens::Palette::STAGE);
            }),
        );
        ctrl2.set_shine_target(&btn_shine);

        let layout_start = Instant::now();
        while (hdr2.height() <= 0
            || hov2.height() <= 0
            || ctrl2.shimmer_area().height() == 0
            || !row_c.is_mapped()
            || win2
                .native()
                .and_then(|n| n.renderer())
                .is_none_or(|r| !r.is_realized()))
            && layout_start.elapsed() < Duration::from_secs(2)
        {
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

        let captured = capture_window_texture(&win2, &ov2)
            .expect("Capture on realized presented window must produce a texture");
        assert!(
            captured.width() >= ov2.width() && captured.height() >= ov2.height(),
            "Captured texture {}x{} must cover scaled overlay {}x{}",
            captured.width(),
            captured.height(),
            ov2.width(),
            ov2.height()
        );

        // Verify snapshot_reveal_nodes produces a circular center hole at tiny t=0.05
        // while keeping corners outside the radius opaque from the captured texture.
        let renderer = win2
            .native()
            .and_then(|n| n.renderer())
            .expect("realized native renderer");
        let w_f = ov2.width() as f64;
        let h_f = ov2.height() as f64;
        let test_snap = gtk4::Snapshot::new();
        snapshot_reveal_nodes(
            &test_snap,
            &captured,
            w_f,
            h_f,
            0.05,
            &tokens::Palette::STAGE,
        );
        let reveal_node = test_snap
            .to_node()
            .expect("snapshot_reveal_nodes must record render nodes");
        let lbounds = graphene::Rect::new(0.0, 0.0, w_f as f32, h_f as f32);
        let rendered = renderer.render_texture(&reveal_node, Some(&lbounds));
        let rw = rendered.width() as usize;
        let rh = rendered.height() as usize;
        assert!(rw > 20 && rh > 20);
        let mut pixels = vec![0u8; rw * rh * 4];
        rendered.download(&mut pixels, rw * 4);
        let center_alpha = pixels[((rh / 2) * rw + (rw / 2)) * 4 + 3];
        let corner_alpha = pixels[(5 * rw + 5) * 4 + 3];
        assert!(
            center_alpha <= 10,
            "Reveal hole center must be transparent, got alpha {center_alpha}"
        );
        assert_eq!(
            corner_alpha, 255,
            "Reveal corner outside radius must keep captured texture opaque, got alpha {corner_alpha}"
        );

        // Verify single-sparkle GSK node budget (<= 3 nodes recursively) and full-frame
        // GSK node count < 150, clipped, with zero Cairo fallback nodes.
        let sp_snap = gtk4::Snapshot::new();
        let sp = Sparkle {
            x: 100.0,
            y: 100.0,
            size: 8.0,
            alpha: 0.8,
            rotation: 20.0,
        };
        append_sparkle_nodes(&sp_snap, sp, 1.0, tokens::Palette::STAGE.gold_light.rgb());
        let sp_node = sp_snap.to_node().expect("sparkle node");
        let sp_ser = String::from_utf8_lossy(sp_node.serialize().as_ref()).to_string();
        let sp_tree_nodes = sp_ser.matches('{').count();
        assert!(
            sp_tree_nodes <= 4,
            "Sparkle render tree node count must stay compact (got {sp_tree_nodes}):\n{sp_ser}"
        );
        let sp_primitives = sp_ser.matches("radial-gradient {").count()
            + sp_ser.matches("fill {").count()
            + sp_ser.matches("color {").count();
        assert!(
            sp_primitives <= 3,
            "Single sparkle must emit at most 3 GSK render nodes recursively, got {sp_primitives}"
        );
        assert!(
            !sp_ser.contains("cairo {"),
            "Sparkle must use GSK nodes only, no cairo:\n{sp_ser}"
        );

        let mut max_total_nodes = 0usize;
        for step in 0..=100 {
            let t = step as f64 / 100.0;
            let frame_snap = gtk4::Snapshot::new();
            snapshot_reveal_nodes(
                &frame_snap,
                &captured,
                1280.0,
                820.0,
                t,
                &tokens::Palette::STAGE,
            );
            if let Some(node) = frame_snap.to_node() {
                let ser = String::from_utf8_lossy(node.serialize().as_ref()).to_string();
                assert!(
                    !ser.contains("cairo {"),
                    "Reveal nodes must use GSK only, no cairo at t={t}"
                );
                if step == 60 {
                    assert!(
                        ser.contains("clip {"),
                        "Reveal overlay must clip effects above mask to window bounds"
                    );
                }
                let total_nodes = ser.matches('{').count();
                assert!(
                    total_nodes < 150,
                    "Total reveal GSK node count at t={t} must stay < 150, got {total_nodes}"
                );
                max_total_nodes = max_total_nodes.max(total_nodes);
            }
        }
        assert!(
            max_total_nodes > 50,
            "Expected peak reveal frame with sparkles to emit > 50 GSK nodes, got {max_total_nodes}"
        );

        // Trigger the unlock sequence: overlay sits on top while Stage skin swaps immediately underneath
        ctrl2.trigger_unlock();
        let swap_time = Instant::now();

        let reveal_widget = ov2.last_child().expect("reveal overlay added");
        assert!(
            reveal_widget.downcast_ref::<RevealOverlay>().is_some(),
            "Added reveal overlay must be a RevealOverlay widget subclass"
        );
        assert!(!reveal_widget.can_target());
        assert!(!reveal_widget.can_focus());
        assert!(
            swapped2.get(),
            "Skin must swap immediately beneath the captured old-texture overlay"
        );
        assert!(
            !style_manager.is_dark(),
            "Stage skin ForceLight scheme must already be applied beneath the reveal overlay"
        );
        assert!(ctrl2.is_effects_active());
        assert!(
            ctrl2.is_header_flash_active(),
            "Reveal must trigger one-shot header flash at start"
        );
        assert!(
            btn_shine.has_css_class("stage-shine"),
            "Build and Flash shine target must receive stage-shine on swap"
        );
        assert!(
            row_a.has_css_class("stage-pre")
                && row_b.has_css_class("stage-pre")
                && row_c.has_css_class("stage-pre")
                && card2.has_css_class("stage-pre"),
            "Visible rows and card must receive stage-pre on swap"
        );

        // Destroy/remove one row mid-stagger to verify weak-ref safety
        list2.remove(&row_ephemeral);
        drop(row_ephemeral);

        // Pump up to 600 ms after swap and verify every row/card has stage-pre removed
        while swap_time.elapsed() < Duration::from_millis(600) {
            glib::MainContext::default().iteration(false);
            if !card2.has_css_class("stage-pre")
                && !row_a.has_css_class("stage-pre")
                && !row_b.has_css_class("stage-pre")
                && !row_c.has_css_class("stage-pre")
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        while glib::MainContext::default().iteration(false) {}
        assert!(
            !card2.has_css_class("stage-pre")
                && !row_a.has_css_class("stage-pre")
                && !row_b.has_css_class("stage-pre")
                && !row_c.has_css_class("stage-pre"),
            "Every row and card must lose stage-pre within 600 ms of swap"
        );

        while (reveal_widget.parent().is_some() || btn_shine.has_css_class("stage-shine"))
            && swap_time.elapsed() < Duration::from_millis(2000)
        {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        while glib::MainContext::default().iteration(false) {}

        // After 750ms reveal duration, RevealOverlay must have been removed cleanly
        assert!(
            reveal_widget.parent().is_none(),
            "RevealOverlay must be removed from overlay after 750ms animation"
        );
        assert!(
            !btn_shine.has_css_class("stage-shine"),
            "stage-shine must be removed cleanly after 700 ms"
        );
        assert_eq!(
            ov2.last_child().as_ref(),
            Some(box2.upcast_ref::<gtk4::Widget>()),
            "Original child must remain cleanly after reveal overlay removal"
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
        gtk4::style_context_remove_provider_for_display(&display, &provider);
        style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
    }

    window.close();
}
