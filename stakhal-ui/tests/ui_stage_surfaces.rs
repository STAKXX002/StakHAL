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

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true)
        .build();
    root_box.append(&header_bar);
    root_box.append(&list_box);

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

    win.close();
    gtk4::style_context_remove_provider_for_display(&display, &provider);
    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
}
