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

    // Pump context for ~50ms (in flight)
    let start_in_flight = std::time::Instant::now();
    while start_in_flight.elapsed() < std::time::Duration::from_millis(50) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let m_start_mid = diagram_child.margin_start();
    println!("Forward t=50ms: margin_start={}", m_start_mid);
    assert!(
        m_start_mid <= 10 && m_start_mid >= 0,
        "In flight margin_start must interpolate downwards"
    );

    // Pump until completion (> 220ms total)
    let start_finish = std::time::Instant::now();
    while start_finish.elapsed() < std::time::Duration::from_millis(220) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let m_start_end = diagram_child.margin_start();
    let m_end_end = diagram_child.margin_end();
    println!("Forward t=250ms (complete): margin_start={}, margin_end={}", m_start_end, m_end_end);
    assert_eq!(m_start_end, 0, "margin_start must reset to 0px on completion (no layout drift)");
    assert_eq!(m_end_end, 0, "margin_end must stay 0px on completion");

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

    // Pump an additional 100ms: verify absolutely no layout jump or drift
    for _ in 0..10 {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(diagram_child.margin_start(), 0);
    assert_eq!(diagram_child.margin_end(), 0);

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
    // Also verify outgoing child (state_diagram) margins were completely cleared
    assert_eq!(diagram_child.margin_start(), 0);
    assert_eq!(diagram_child.margin_end(), 0);

    // Pump until completion (> 220ms total)
    let start_back_finish = std::time::Instant::now();
    while start_back_finish.elapsed() < std::time::Duration::from_millis(250) {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let m_start_back_end = overview_child.margin_start();
    let m_end_back_end = overview_child.margin_end();
    println!("Backward t=250ms (complete): margin_start={}, margin_end={}", m_start_back_end, m_end_back_end);
    assert_eq!(m_start_back_end, 0, "margin_start must be 0px on completion");
    assert_eq!(m_end_back_end, 0, "margin_end must be 0px on completion (no layout drift)");

    // Save visual frames to artifacts directory
    let artifact_dir = std::path::PathBuf::from("/home/stakxx002/.gemini/antigravity-ide/brain/e21edbbd-844e-44ef-9dfa-1af3c8e3a19b");
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
}
