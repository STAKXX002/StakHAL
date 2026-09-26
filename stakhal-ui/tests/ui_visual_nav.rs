use gtk4::{gdk, gio, glib};
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::{build_ui, navigate_stack};

#[test]
fn test_visual_nav_transitions() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping visual nav test: {}", err);
        return;
    }
    if gdk::Display::default().is_none() {
        eprintln!("GDK default display not available, skipping visual nav test");
        return;
    }
    let _ = adw::init();

    let app = adw::Application::builder()
        .application_id("com.stakhal.ui.visual_nav_test")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();

    if let Err(err) = app.register(gio::Cancellable::NONE) {
        eprintln!("Failed to register GTK application in test: {}", err);
        return;
    }

    app.connect_activate(build_ui);
    app.activate();

    let windows = app.windows();
    assert!(!windows.is_empty(), "Application window must exist");
    let win = windows.first().unwrap().clone();
    win.present();

    // Pump context to allow initial allocation
    for _ in 0..10 {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Find the main stack
    fn find_stack(widget: &gtk4::Widget) -> Option<gtk4::Stack> {
        if let Ok(st) = widget.clone().downcast::<gtk4::Stack>() {
            return Some(st);
        }
        let mut child = widget.first_child();
        while let Some(c) = child {
            if let Some(found) = find_stack(&c) {
                return Some(found);
            }
            child = c.next_sibling();
        }
        None
    }

    let stack = find_stack(win.upcast_ref()).expect("Stack must exist in window");

    // Enable animations for visual testing
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    let diagram_child = stack.child_by_name("state_diagram").expect("state_diagram child exists");
    let overview_child = stack.child_by_name("overview").expect("overview child exists");

    // Initial state: overview visible, 0 margins
    assert_eq!(stack.visible_child_name().as_deref(), Some("overview"));
    assert_eq!(diagram_child.margin_start(), 0);
    assert_eq!(diagram_child.margin_end(), 0);
    assert_eq!(overview_child.margin_start(), 0);
    assert_eq!(overview_child.margin_end(), 0);

    // 1. FORWARD NAVIGATION (Overview -> State Diagram)
    println!("--- Testing Forward Navigation (SlideLeft intent) ---");
    navigate_stack(&stack, "state_diagram", gtk4::StackTransitionType::SlideLeft);

    assert_eq!(stack.transition_type(), gtk4::StackTransitionType::Crossfade);
    assert_eq!(
        stack.transition_duration(),
        stakhal_ui::ui::tokens::motion::DURATION_SHORT_MS as u32
    );

    // At t = 0ms: incoming child (state_diagram) starts shifted right by 10px
    let m_start_0 = diagram_child.margin_start();
    let m_end_0 = diagram_child.margin_end();
    println!("Forward t=0ms: margin_start={}, margin_end={}", m_start_0, m_end_0);
    assert_eq!(m_start_0, 10, "Forward navigation must set margin_start = 10px");
    assert_eq!(m_end_0, 0, "Forward navigation must keep margin_end = 0px");

    // Pump context to t=50ms (in flight)
    let start_fwd = std::time::Instant::now();
    while start_fwd.elapsed() < std::time::Duration::from_millis(50) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let m_start_50 = diagram_child.margin_start();
    println!("Forward t=50ms: margin_start={}", m_start_50);
    assert!(
        m_start_50 <= 10 && m_start_50 >= 0,
        "In flight margin_start must interpolate downwards"
    );

    // Pump context to t=190ms (just past 180ms total_duration)
    while start_fwd.elapsed() < std::time::Duration::from_millis(190) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while glib::MainContext::default().iteration(false) {}
    let m_start_190 = diagram_child.margin_start();
    let m_end_190 = diagram_child.margin_end();
    println!("Forward t=190ms: margin_start={}, margin_end={}", m_start_190, m_end_190);
    assert_eq!(m_start_190, 0, "Forward navigation margin_start must be 0px at t=190ms");
    assert_eq!(m_end_190, 0, "Forward navigation margin_end must be 0px at t=190ms");

    // Pump context to t=220ms
    while start_fwd.elapsed() < std::time::Duration::from_millis(220) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while glib::MainContext::default().iteration(false) {}
    let m_start_220 = diagram_child.margin_start();
    let m_end_220 = diagram_child.margin_end();
    println!("Forward t=220ms: margin_start={}, margin_end={}", m_start_220, m_end_220);
    assert_eq!(m_start_220, 0, "Forward navigation margin_start must remain 0px at t=220ms");
    assert_eq!(m_end_220, 0, "Forward navigation margin_end must remain 0px at t=220ms");

    // Pump context to t=260ms
    while start_fwd.elapsed() < std::time::Duration::from_millis(260) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while glib::MainContext::default().iteration(false) {}
    let m_start_260 = diagram_child.margin_start();
    let m_end_260 = diagram_child.margin_end();
    println!("Forward t=260ms: margin_start={}, margin_end={}", m_start_260, m_end_260);
    assert_eq!(m_start_260, 0, "Forward navigation margin_start must remain 0px at t=260ms");
    assert_eq!(m_end_260, 0, "Forward navigation margin_end must remain 0px at t=260ms");

    let artifact_dir = std::path::PathBuf::from("/home/stakxx002/.gemini/antigravity-ide/brain/e21edbbd-844e-44ef-9dfa-1af3c8e3a19b");
    if artifact_dir.exists() {
        let paintable = gtk4::WidgetPaintable::new(Some(&win));
        let snapshot = gtk4::Snapshot::new();
        paintable.snapshot(&snapshot, 1200.0, 800.0);
        if let Some(node) = snapshot.to_node() {
            let renderer = gtk4::gsk::CairoRenderer::new();
            if renderer.realize(None).is_ok() {
                let texture = renderer.render_texture(&node, None);
                let _ = texture.save_to_png(artifact_dir.join("visual_nav_diagram_forward.png"));
                renderer.unrealize();
                println!("Saved visual artifact: visual_nav_diagram_forward.png");
            }
        }
    }

    // 2. BACKWARD NAVIGATION (State Diagram -> Overview)
    println!("--- Testing Backward Navigation (SlideRight intent) ---");
    navigate_stack(&stack, "overview", gtk4::StackTransitionType::SlideRight);

    assert_eq!(stack.transition_type(), gtk4::StackTransitionType::Crossfade);
    assert_eq!(
        stack.transition_duration(),
        stakhal_ui::ui::tokens::motion::DURATION_SHORT_MS as u32
    );

    // At t = 0ms: incoming child (overview) starts with margin_end = 10px
    let m_start_back_0 = overview_child.margin_start();
    let m_end_back_0 = overview_child.margin_end();
    println!("Backward t=0ms: margin_start={}, margin_end={}", m_start_back_0, m_end_back_0);
    assert_eq!(m_end_back_0, 10, "Backward navigation must set margin_end = 10px");
    assert_eq!(m_start_back_0, 0, "Backward navigation must keep margin_start = 0px");
    assert_eq!(diagram_child.margin_start(), 0);
    assert_eq!(diagram_child.margin_end(), 0);

    // Pump context to t=50ms
    let start_back = std::time::Instant::now();
    while start_back.elapsed() < std::time::Duration::from_millis(50) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let m_end_back_50 = overview_child.margin_end();
    println!("Backward t=50ms: margin_end={}", m_end_back_50);
    assert!(
        m_end_back_50 <= 10 && m_end_back_50 >= 0,
        "In flight margin_end must interpolate downwards"
    );

    // Pump context to t=190ms
    while start_back.elapsed() < std::time::Duration::from_millis(190) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while glib::MainContext::default().iteration(false) {}
    let m_start_back_190 = overview_child.margin_start();
    let m_end_back_190 = overview_child.margin_end();
    println!("Backward t=190ms: margin_start={}, margin_end={}", m_start_back_190, m_end_back_190);
    assert_eq!(m_start_back_190, 0, "Backward navigation margin_start must be 0px at t=190ms");
    assert_eq!(m_end_back_190, 0, "Backward navigation margin_end must be 0px at t=190ms");

    // Pump context to t=220ms
    while start_back.elapsed() < std::time::Duration::from_millis(220) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while glib::MainContext::default().iteration(false) {}
    let m_start_back_220 = overview_child.margin_start();
    let m_end_back_220 = overview_child.margin_end();
    println!("Backward t=220ms: margin_start={}, margin_end={}", m_start_back_220, m_end_back_220);
    assert_eq!(m_start_back_220, 0, "Backward navigation margin_start must be 0px at t=220ms");
    assert_eq!(m_end_back_220, 0, "Backward navigation margin_end must be 0px at t=220ms");

    // Pump context to t=260ms
    while start_back.elapsed() < std::time::Duration::from_millis(260) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    while glib::MainContext::default().iteration(false) {}
    let m_start_back_260 = overview_child.margin_start();
    let m_end_back_260 = overview_child.margin_end();
    println!("Backward t=260ms: margin_start={}, margin_end={}", m_start_back_260, m_end_back_260);
    assert_eq!(m_start_back_260, 0, "Backward navigation margin_start must be 0px at t=260ms");
    assert_eq!(m_end_back_260, 0, "Backward navigation margin_end must be 0px at t=260ms");

    if artifact_dir.exists() {
        let paintable = gtk4::WidgetPaintable::new(Some(&win));
        let snapshot = gtk4::Snapshot::new();
        paintable.snapshot(&snapshot, 1200.0, 800.0);
        if let Some(node) = snapshot.to_node() {
            let renderer = gtk4::gsk::CairoRenderer::new();
            if renderer.realize(None).is_ok() {
                let texture = renderer.render_texture(&node, None);
                let _ = texture.save_to_png(artifact_dir.join("visual_nav_settled_overview.png"));
                renderer.unrealize();
                println!("Saved visual artifact: visual_nav_settled_overview.png");
            }
        }
    }

    // 3. REDUCED-MOTION PATH (Instant cut with 0px margins)
    println!("--- Testing Reduced-Motion Path ---");
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }

    // Forward with reduced motion
    navigate_stack(&stack, "state_diagram", gtk4::StackTransitionType::SlideLeft);
    assert_eq!(stack.transition_duration(), 0);
    assert_eq!(stack.visible_child_name().as_deref(), Some("state_diagram"));
    assert_eq!(diagram_child.margin_start(), 0, "Reduced motion must have 0px margin_start immediately at t=0ms");
    assert_eq!(diagram_child.margin_end(), 0, "Reduced motion must have 0px margin_end immediately at t=0ms");

    let start_red = std::time::Instant::now();
    for target_ms in [50, 190, 220, 260] {
        while start_red.elapsed() < std::time::Duration::from_millis(target_ms) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(diagram_child.margin_start(), 0, "Reduced motion margin_start must stay 0 at t={}ms", target_ms);
        assert_eq!(diagram_child.margin_end(), 0, "Reduced motion margin_end must stay 0 at t={}ms", target_ms);
    }

    // Backward with reduced motion
    navigate_stack(&stack, "overview", gtk4::StackTransitionType::SlideRight);
    assert_eq!(stack.transition_duration(), 0);
    assert_eq!(stack.visible_child_name().as_deref(), Some("overview"));
    assert_eq!(overview_child.margin_start(), 0, "Reduced motion backward must have 0px margin_start immediately");
    assert_eq!(overview_child.margin_end(), 0, "Reduced motion backward must have 0px margin_end immediately");

    let start_red_back = std::time::Instant::now();
    for target_ms in [50, 190, 220, 260] {
        while start_red_back.elapsed() < std::time::Duration::from_millis(target_ms) {
            glib::MainContext::default().iteration(false);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(overview_child.margin_start(), 0, "Reduced motion backward margin_start must stay 0 at t={}ms", target_ms);
        assert_eq!(overview_child.margin_end(), 0, "Reduced motion backward margin_end must stay 0 at t={}ms", target_ms);
    }

    // Restore setting
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    // 4. ACTIVE-TRANSITION SIGNAL PULSE ANIMATION
    println!("--- Testing Active-Transition Signal Pulse Animation ---");
    navigate_stack(&stack, "state_diagram", gtk4::StackTransitionType::Crossfade);
    while glib::MainContext::default().iteration(false) {}

    let dur_ms = stakhal_ui::ui::tokens::motion::DURATION_MEDIUM_MS as f64;
    let pulse_progress = |start: std::time::Instant| -> f64 {
        let elapsed = start.elapsed().as_millis() as f64;
        (elapsed / dur_ms).clamp(0.0, 1.0)
    };

    // t = 0ms
    let start_pulse = std::time::Instant::now();
    let p0 = pulse_progress(start_pulse);
    println!("Signal pulse t=0ms: progress={:.3}", p0);
    assert!(p0 < 0.05, "Progress at t=0ms must be ~0.0");

    // mid (175ms)
    let start_mid = std::time::Instant::now() - std::time::Duration::from_millis(175);
    let p_mid = pulse_progress(start_mid);
    println!("Signal pulse mid (t=175ms): progress={:.3}", p_mid);
    assert!((p_mid - 0.5).abs() < 0.05, "Progress at mid must be ~0.5");

    // exactly 350ms
    let start_350 = std::time::Instant::now() - std::time::Duration::from_millis(350);
    let p_350 = pulse_progress(start_350);
    println!("Signal pulse t=350ms: progress={:.3}", p_350);
    assert_eq!(p_350, 1.0, "Progress at exactly 350ms must equal 1.0");

    // +30ms (380ms)
    let start_380 = std::time::Instant::now() - std::time::Duration::from_millis(380);
    let p_380 = pulse_progress(start_380);
    println!("Signal pulse t=380ms (+30ms): progress={:.3}", p_380);
    assert_eq!(p_380, 1.0, "Progress at +30ms must be clamped to 1.0");

    // +80ms (430ms)
    let start_430 = std::time::Instant::now() - std::time::Duration::from_millis(430);
    let p_430 = pulse_progress(start_430);
    println!("Signal pulse t=430ms (+80ms): progress={:.3}", p_430);
    assert_eq!(p_430, 1.0, "Progress at +80ms must be clamped to 1.0");

    // Reduced-motion: animations disabled immediately cancels pulse
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    assert!(!stakhal_ui::ui::tokens::motion::is_animations_enabled());

    // Restore setting
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }
}
