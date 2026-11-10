use gtk4::cairo;
use gtk4::gdk;
use gtk4::glib;
use gtk4::gsk;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::{apply_skin, build_css, ui};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

fn sample_widget_rgba(widget: &impl IsA<gtk4::Widget>, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let w = widget.width().max(1);
    let h = widget.height().max(1);
    let paintable = gtk4::WidgetPaintable::new(Some(widget));
    let snapshot = gtk4::Snapshot::new();
    paintable.snapshot(&snapshot, w as f64, h as f64);
    let node = snapshot
        .to_node()
        .expect("widget paintable produced render node");

    let mut surface =
        cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).expect("create image surface");
    {
        let cr = cairo::Context::new(&surface).expect("create cairo context");
        gsk::RenderNode::draw(&node, &cr);
    }
    surface.flush();

    let stride = surface.stride() as usize;
    let data = surface.data().expect("surface pixel data");
    let sx = x.clamp(0, w - 1) as usize;
    let sy = y.clamp(0, h - 1) as usize;
    let offset = sy * stride + sx * 4;
    let b = data[offset];
    let g = data[offset + 1];
    let r = data[offset + 2];
    let a = data[offset + 3];
    (r, g, b, a)
}

fn to_u8_rgb(rgb: (f64, f64, f64)) -> (u8, u8, u8) {
    (
        (rgb.0 * 255.0).round() as u8,
        (rgb.1 * 255.0).round() as u8,
        (rgb.2 * 255.0).round() as u8,
    )
}

fn serialize_widget_node(widget: &impl IsA<gtk4::Widget>) -> String {
    let w = widget.width().max(1);
    let h = widget.height().max(1);
    let paintable = gtk4::WidgetPaintable::new(Some(widget));
    let snapshot = gtk4::Snapshot::new();
    paintable.snapshot(&snapshot, w as f64, h as f64);
    let node = snapshot
        .to_node()
        .expect("widget paintable produced render node");
    String::from_utf8_lossy(node.serialize().as_ref()).to_string()
}

fn pump_for(duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        while glib::MainContext::default().iteration(false) {}
        std::thread::sleep(Duration::from_millis(10));
    }
    while glib::MainContext::default().iteration(false) {}
}

#[test]
fn test_stage_css_parses_cleanly_and_colors_adwaita_surfaces() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping stage surface test: {err}");
        return;
    }
    let Some(display) = gdk::Display::default() else {
        eprintln!("GDK default display not available, skipping stage surface test");
        return;
    };
    let _ = adw::init();

    let parse_errors: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let provider = gtk4::CssProvider::new();
    let errs_cb = Rc::clone(&parse_errors);
    provider.connect_parsing_error(move |_, section, error| {
        errs_cb.borrow_mut().push(format!(
            "line {}: {error}",
            section.start_location().lines() + 1
        ));
    });

    provider.load_from_string(&build_css(&ui::tokens::Palette::DEFAULT));
    assert!(
        parse_errors.borrow().is_empty(),
        "DEFAULT CSS parse errors: {:?}",
        parse_errors.borrow()
    );

    provider.load_from_string(&build_css(&ui::tokens::Palette::STAGE));
    assert!(
        parse_errors.borrow().is_empty(),
        "STAGE CSS parse errors (including mix() and :root): {:?}",
        parse_errors.borrow()
    );

    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    apply_skin(&provider, ui::tokens::Palette::STAGE);

    let header_bar = adw::HeaderBar::new();
    let list_box = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();
    let row = ui::main_panel::create_peripheral_row("USART2", Some("Asynchronous"), 6);
    list_box.append(&row);

    let btn_anim = gtk4::Button::builder()
        .label("Build & Flash")
        .css_classes(vec![
            "stakhal-btn".to_string(),
            "suggested-action".to_string(),
        ])
        .build();
    let chip = gtk4::Label::builder()
        .label("ACTIVE")
        .css_classes(vec![
            "card".to_string(),
            "status-chip".to_string(),
            "status-active".to_string(),
        ])
        .build();
    let controls_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(12)
        .margin_top(12)
        .margin_start(12)
        .build();
    controls_row.append(&btn_anim);
    controls_row.append(&chip);

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true)
        .build();
    root_box.append(&header_bar);
    root_box.append(&list_box);
    root_box.append(&controls_row);

    let win = adw::ApplicationWindow::builder()
        .default_width(600)
        .default_height(400)
        .content(&root_box)
        .build();
    win.present();

    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(120)
        || header_bar.height() <= 0
        || row.height() <= 0
    {
        glib::MainContext::default().iteration(false);
        std::thread::sleep(Duration::from_millis(10));
        if start.elapsed() > Duration::from_secs(2) {
            break;
        }
    }
    while glib::MainContext::default().iteration(false) {}

    assert!(header_bar.width() > 100 && header_bar.height() > 20);
    assert!(row.width() > 100 && row.height() > 20);

    let header_px = sample_widget_rgba(&header_bar, 24, header_bar.height() / 2);
    let expected_header = to_u8_rgb(ui::tokens::Palette::STAGE.header.rgb());
    assert!(
        header_px.0.abs_diff(expected_header.0) <= 2
            && header_px.1.abs_diff(expected_header.1) <= 2
            && header_px.2.abs_diff(expected_header.2) <= 2
            && header_px.3 >= 250,
        "adw::HeaderBar background {:?} must match Stage header {:?}",
        header_px,
        expected_header
    );

    let row_px = sample_widget_rgba(&row, row.width() / 2, row.height() - 4);
    let expected_row = to_u8_rgb(ui::tokens::Palette::STAGE.bg_panel.rgb());
    assert!(
        row_px.0.abs_diff(expected_row.0) <= 2
            && row_px.1.abs_diff(expected_row.1) <= 2
            && row_px.2.abs_diff(expected_row.2) <= 2
            && row_px.3 >= 250,
        "adw::ActionRow background {:?} must match Stage bg_panel {:?}",
        row_px,
        expected_row
    );

    let bottom_px = sample_widget_rgba(&win, win.width() / 2, win.height() - 16);
    assert!(
        bottom_px.2 > bottom_px.0 + 10 && bottom_px.0 >= 200 && bottom_px.2 >= 230,
        "Window bottom gradient pixel {:?} must show pale azure Stage tint",
        bottom_px
    );

    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    // Verify live GTK honors CSS transform on hover (:hover translateY(-1px)) and active (:active scale(0.98))
    let ser_btn_idle = serialize_widget_node(&btn_anim);
    btn_anim.set_state_flags(gtk4::StateFlags::PRELIGHT, false);
    pump_for(Duration::from_millis(220));
    let ser_btn_hover = serialize_widget_node(&btn_anim);
    assert!(
        ser_btn_hover != ser_btn_idle && ser_btn_hover.contains("transform {"),
        "GTK must honor Stage hover transform on button:\n{ser_btn_hover}"
    );
    btn_anim.unset_state_flags(gtk4::StateFlags::PRELIGHT);
    btn_anim.set_state_flags(gtk4::StateFlags::ACTIVE, false);
    pump_for(Duration::from_millis(160));
    let ser_btn_active = serialize_widget_node(&btn_anim);
    assert!(
        ser_btn_active != ser_btn_hover && ser_btn_active.contains("scale("),
        "GTK must honor Stage active scale(0.98) transform on button:\n{ser_btn_active}"
    );
    btn_anim.unset_state_flags(gtk4::StateFlags::ACTIVE);
    pump_for(Duration::from_millis(160));

    // Verify @keyframes stage-active-breath runs ONLY on .status-active and stops when removed
    let ser_active_t0 = serialize_widget_node(&chip);
    pump_for(Duration::from_millis(280));
    let ser_active_t1 = serialize_widget_node(&chip);
    assert_ne!(
        ser_active_t0, ser_active_t1,
        ".status-active @keyframes stage-active-breath must animate across frames"
    );

    for static_class in ["status-idle", "status-ready", "status-error"] {
        chip.remove_css_class("status-active");
        chip.add_css_class(static_class);
        pump_for(Duration::from_millis(220));
        let s0 = serialize_widget_node(&chip);
        pump_for(Duration::from_millis(260));
        let s1 = serialize_widget_node(&chip);
        assert_eq!(
            s0, s1,
            "Static status class {static_class} must have zero animation"
        );
        chip.remove_css_class(static_class);
    }

    // Reduced motion must halt CSS @keyframes animation
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    chip.add_css_class("status-active");
    pump_for(Duration::from_millis(80));
    let ser_rm0 = serialize_widget_node(&chip);
    pump_for(Duration::from_millis(220));
    let ser_rm1 = serialize_widget_node(&chip);
    assert_eq!(
        ser_rm0, ser_rm1,
        "Reduced motion must disable Stage CSS @keyframes animation"
    );
    if let Some(settings) = gtk4::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }

    win.close();
    gtk4::style_context_remove_provider_for_display(&display, &provider);
    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
}
