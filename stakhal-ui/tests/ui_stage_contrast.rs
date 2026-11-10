use gtk4::cairo;
use gtk4::gdk;
use gtk4::glib;
use gtk4::gsk;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_ui::{apply_skin, ui};
use std::collections::HashMap;
use std::time::{Duration, Instant};

fn srgb_channel_to_linear(c: u8) -> f64 {
    let s = c as f64 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance_rgb(rgb: (u8, u8, u8)) -> f64 {
    0.2126 * srgb_channel_to_linear(rgb.0)
        + 0.7152 * srgb_channel_to_linear(rgb.1)
        + 0.0722 * srgb_channel_to_linear(rgb.2)
}

fn wcag_contrast_rgb(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let la = relative_luminance_rgb(a);
    let lb = relative_luminance_rgb(b);
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn render_widget_to_surface(widget: &impl IsA<gtk4::Widget>) -> cairo::ImageSurface {
    let w = widget.width().max(1);
    let h = widget.height().max(1);
    let paintable = gtk4::WidgetPaintable::new(Some(widget));
    let snapshot = gtk4::Snapshot::new();
    paintable.snapshot(&snapshot, w as f64, h as f64);
    let node = snapshot
        .to_node()
        .expect("widget paintable produced render node");

    let surface =
        cairo::ImageSurface::create(cairo::Format::ARgb32, w, h).expect("create image surface");
    {
        let cr = cairo::Context::new(&surface).expect("create cairo context");
        gsk::RenderNode::draw(&node, &cr);
    }
    surface.flush();
    surface
}

fn surface_pixel(surface: &mut cairo::ImageSurface, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let w = surface.width();
    let h = surface.height();
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

#[derive(Debug, Clone, Copy)]
struct RectPx {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

fn widget_bounds_in(
    widget: &impl IsA<gtk4::Widget>,
    root: &impl IsA<gtk4::Widget>,
    inset_x: i32,
    inset_y: i32,
) -> RectPx {
    let bounds = widget
        .compute_bounds(root)
        .expect("widget must have bounds relative to root");
    let rx0 = bounds.x().round() as i32 + inset_x;
    let ry0 = bounds.y().round() as i32 + inset_y;
    let rx1 = (bounds.x() + bounds.width()).round() as i32 - inset_x;
    let ry1 = (bounds.y() + bounds.height()).round() as i32 - inset_y;
    RectPx {
        x0: rx0.clamp(0, root.width() - 1),
        y0: ry0.clamp(0, root.height() - 1),
        x1: rx1.clamp(rx0 + 1, root.width()),
        y1: ry1.clamp(ry0 + 1, root.height()),
    }
}

/// Samples the dominant background pixel (mode RGB) and the highest-contrast text
/// pixel within `rect` on the already-rendered surface.
fn sample_fg_and_bg_in_rect(
    surface: &mut cairo::ImageSurface,
    rect: RectPx,
) -> ((u8, u8, u8), (u8, u8, u8), f64) {
    let mut counts: HashMap<(u8, u8, u8), usize> = HashMap::new();
    let mut pixels: Vec<(u8, u8, u8)> = Vec::new();
    for y in rect.y0..rect.y1 {
        for x in rect.x0..rect.x1 {
            let (r, g, b, _a) = surface_pixel(surface, x, y);
            let rgb = (r, g, b);
            *counts.entry(rgb).or_insert(0) += 1;
            pixels.push(rgb);
        }
    }
    let bg = counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .expect("sample rect must contain pixels")
        .0;

    let bg_lum = relative_luminance_rgb(bg);
    let fg = pixels
        .into_iter()
        .max_by(|a, b| {
            let da = (relative_luminance_rgb(*a) - bg_lum).abs();
            let db = (relative_luminance_rgb(*b) - bg_lum).abs();
            da.total_cmp(&db)
        })
        .expect("sample rect must contain pixels");

    let contrast = wcag_contrast_rgb(fg, bg);
    (fg, bg, contrast)
}

fn find_descendant<P>(root: &gtk4::Widget, pred: &P) -> Option<gtk4::Widget>
where
    P: Fn(&gtk4::Widget) -> bool,
{
    if pred(root) {
        return Some(root.clone());
    }
    let mut child = root.first_child();
    while let Some(c) = child {
        if let Some(found) = find_descendant(&c, pred) {
            return Some(found);
        }
        child = c.next_sibling();
    }
    None
}

fn pump_events(duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        while glib::MainContext::default().iteration(false) {}
        std::thread::sleep(Duration::from_millis(10));
    }
    while glib::MainContext::default().iteration(false) {}
}

fn assert_min_contrast(
    surface: &mut cairo::ImageSurface,
    widget: &impl IsA<gtk4::Widget>,
    root: &impl IsA<gtk4::Widget>,
    inset: i32,
    min_ratio: f64,
    label: &str,
) -> ((u8, u8, u8), (u8, u8, u8), f64) {
    let rect = widget_bounds_in(widget, root, inset, inset);
    let (fg, bg, cr) = sample_fg_and_bg_in_rect(surface, rect);
    eprintln!("Stage rendered {label}: fg={fg:?} bg={bg:?} contrast={cr:.2}");
    assert!(
        cr >= min_ratio,
        "{label} contrast {cr:.2} < {min_ratio} (fg={fg:?}, bg={bg:?})"
    );
    (fg, bg, cr)
}

#[test]
fn test_stage_rendered_wcag_contrast() {
    if let Err(err) = gtk4::init() {
        eprintln!("GTK display not available, skipping stage contrast test: {err}");
        return;
    }
    let Some(display) = gdk::Display::default() else {
        eprintln!("GDK default display not available, skipping stage contrast test");
        return;
    };
    let _ = adw::init();

    let provider = gtk4::CssProvider::new();
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    apply_skin(&provider, ui::tokens::Palette::STAGE);

    let header_bar = adw::HeaderBar::new();
    header_bar.set_show_end_title_buttons(false);
    let header_right_lbl = gtk4::Label::builder()
        .label("Header Light End TEXT")
        .margin_end(8)
        .build();
    header_bar.pack_end(&header_right_lbl);

    let card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(16)
        .margin_end(16)
        .css_classes(vec!["card".to_string()])
        .build();

    let inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(8)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(14)
        .margin_end(14)
        .build();
    card.append(&inner);

    let lbl_normal = gtk4::Label::builder()
        .label("Normal Stage Label TEXT")
        .halign(gtk4::Align::Start)
        .build();

    let lbl_disabled = gtk4::Label::builder()
        .label("Disabled Stage Label TEXT")
        .halign(gtk4::Align::Start)
        .sensitive(false)
        .build();

    let btn_flat_disabled = gtk4::Button::builder()
        .label("Disabled Flat Action")
        .halign(gtk4::Align::Start)
        .sensitive(false)
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .build();

    let lbl_dim = gtk4::Label::builder()
        .label("Dim Secondary Label TEXT")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string()])
        .build();

    let entry = gtk4::Entry::builder()
        .placeholder_text("Enter command placeholder TEXT")
        .hexpand(true)
        .can_focus(false)
        .build();

    let buttons_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .build();

    let btn_suggested = gtk4::Button::builder()
        .label("Suggested Action TEXT")
        .height_request(40)
        .width_request(190)
        .css_classes(vec![
            "stakhal-btn".to_string(),
            "suggested-action".to_string(),
        ])
        .build();

    let btn_secondary = gtk4::Button::builder()
        .label("Secondary Action TEXT")
        .height_request(40)
        .css_classes(vec!["stakhal-btn".to_string()])
        .build();

    let btn_secondary_hover = gtk4::Button::builder()
        .label("Secondary Hovered TEXT")
        .height_request(40)
        .css_classes(vec!["stakhal-btn".to_string()])
        .build();
    btn_secondary_hover.set_state_flags(gtk4::StateFlags::PRELIGHT, false);

    let btn_secondary_flat = gtk4::Button::builder()
        .label("Flat Secondary TEXT")
        .height_request(40)
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .build();

    let icon_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .height_request(40)
        .css_classes(vec!["stakhal-btn".to_string(), "flat".to_string()])
        .build();

    buttons_row.append(&btn_suggested);
    buttons_row.append(&btn_secondary);
    buttons_row.append(&btn_secondary_hover);
    buttons_row.append(&btn_secondary_flat);
    buttons_row.append(&icon_btn);

    let lbl_section = gtk4::Label::builder()
        .label("[ PERIPHERALS ]")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["title-4".to_string()])
        .build();

    let chips_row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();

    let make_chip = |text: &str, status_class: &str| {
        gtk4::Label::builder()
            .label(text)
            .css_classes(vec![
                "card".to_string(),
                "caption".to_string(),
                "data-mono".to_string(),
                status_class.to_string(),
            ])
            .build()
    };
    let chip_idle = make_chip("IDLE", "status-idle");
    let chip_ready = make_chip("READY", "status-ready");
    let chip_active = make_chip("BUSY", "status-active");
    let chip_error = make_chip("ERROR", "status-error");
    chips_row.append(&chip_idle);
    chips_row.append(&chip_ready);
    chips_row.append(&chip_active);
    chips_row.append(&chip_error);

    let list_box = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .build();
    let row_hovered = ui::main_panel::create_peripheral_row("USART2", Some("Asynchronous"), 6);
    row_hovered.set_state_flags(gtk4::StateFlags::PRELIGHT, false);
    let row_even = ui::main_panel::create_peripheral_row("SPI1", Some("Master Mode"), 4);
    let row_selected = ui::main_panel::create_peripheral_row("CAN1", Some("Loopback Mode"), 5);
    list_box.append(&row_hovered);
    list_box.append(&row_even);
    list_box.append(&row_selected);
    list_box.select_row(Some(&row_selected));

    inner.append(&lbl_section);
    inner.append(&lbl_normal);
    inner.append(&lbl_disabled);
    inner.append(&btn_flat_disabled);
    inner.append(&lbl_dim);
    inner.append(&entry);
    inner.append(&buttons_row);
    inner.append(&chips_row);
    inner.append(&list_box);

    let bottom_spacer = gtk4::Box::builder().vexpand(true).build();
    let grad_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .halign(gtk4::Align::Start)
        .margin_start(16)
        .margin_bottom(12)
        .build();
    let lbl_grad_primary = gtk4::Label::builder()
        .label("Bottom Gradient Primary TEXT")
        .halign(gtk4::Align::Start)
        .build();
    let lbl_grad_muted = gtk4::Label::builder()
        .label("Bottom Gradient Muted TEXT")
        .halign(gtk4::Align::Start)
        .css_classes(vec!["dim-label".to_string()])
        .build();
    grad_box.append(&lbl_grad_primary);
    grad_box.append(&lbl_grad_muted);

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true)
        .build();
    root_box.append(&header_bar);
    root_box.append(&card);
    root_box.append(&bottom_spacer);
    root_box.append(&grad_box);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&root_box));

    let win = adw::ApplicationWindow::builder()
        .default_width(960)
        .default_height(700)
        .content(&toast_overlay)
        .build();
    win.present();

    let toast = adw::Toast::builder()
        .title("Stage Skin Activated")
        .button_label("Dismiss")
        .timeout(0)
        .build();
    toast_overlay.add_toast(toast);

    // Wait long enough for libadwaita toast show transition and layout to settle completely.
    pump_events(Duration::from_millis(500));

    let find_lbl = |parent: &gtk4::Widget, txt: &str| {
        find_descendant(parent, &|w| {
            w.downcast_ref::<gtk4::Label>()
                .is_some_and(|l| l.text() == txt)
        })
        .unwrap_or_else(|| panic!("expected label with text {txt:?}"))
    };

    let hover_title_lbl = find_lbl(row_hovered.upcast_ref(), "USART2");
    let hover_sub_lbl = find_lbl(row_hovered.upcast_ref(), "Asynchronous");
    let even_title_lbl = find_lbl(row_even.upcast_ref(), "SPI1");
    let even_sub_lbl = find_lbl(row_even.upcast_ref(), "Master Mode");
    let sel_title_lbl = find_lbl(row_selected.upcast_ref(), "CAN1");
    let sel_sub_lbl = find_lbl(row_selected.upcast_ref(), "Loopback Mode");
    let sug_btn_lbl = find_lbl(btn_suggested.upcast_ref(), "Suggested Action TEXT");

    let toast_widget = find_descendant(toast_overlay.upcast_ref(), &|w| w.css_name() == "toast")
        .expect("ToastOverlay must contain mapped toast widget");
    assert!(
        toast_widget.is_mapped() && toast_widget.height() > 20 && toast_widget.width() > 80,
        "toast must be mapped and sized cleanly ({}x{})",
        toast_widget.width(),
        toast_widget.height()
    );

    let toast_label = find_lbl(&toast_widget, "Stage Skin Activated");
    let toast_btn_label = find_lbl(&toast_widget, "Dismiss");

    let mut surface = render_widget_to_surface(&win);

    // 1. Section title (.title-4): sky gradient fill, gold left accent, primary text (>= 4.5)
    let sec_outer = widget_bounds_in(&lbl_section, &win, 0, 0);
    let sec_inner = widget_bounds_in(&lbl_section, &win, 4, 2);
    let sec_bg_px = surface_pixel(&mut surface, sec_inner.x0 + 2, sec_outer.y0 + 2);
    let sec_bg = (sec_bg_px.0, sec_bg_px.1, sec_bg_px.2);
    let mut sec_fg = sec_bg;
    let mut best_lum = relative_luminance_rgb(sec_bg);
    for y in sec_inner.y0..sec_inner.y1 {
        for x in sec_inner.x0..sec_inner.x1 {
            let p = surface_pixel(&mut surface, x, y);
            let rgb = (p.0, p.1, p.2);
            let l = relative_luminance_rgb(rgb);
            if l < best_lum {
                best_lum = l;
                sec_fg = rgb;
            }
        }
    }
    let sec_cr = wcag_contrast_rgb(sec_fg, sec_bg);
    eprintln!(
        "Stage rendered section title (.title-4): fg={sec_fg:?} bg={sec_bg:?} contrast={sec_cr:.2}"
    );
    assert!(
        sec_cr >= 4.5,
        "section title contrast {sec_cr:.2} < 4.5 (fg={sec_fg:?}, bg={sec_bg:?})"
    );
    let sec_gold_px = surface_pixel(
        &mut surface,
        sec_outer.x0 + 1,
        (sec_outer.y0 + sec_outer.y1) / 2,
    );
    assert!(
        sec_gold_px.0 > sec_gold_px.2 + 40,
        "section title left border {sec_gold_px:?} must render @gold"
    );
    let (norm_fg, _, _) =
        assert_min_contrast(&mut surface, &lbl_normal, &win, 0, 4.5, "normal label");

    // 2. Disabled label and disabled flat button (>= 3.0 AND visibly lighter than enabled label)
    let (dis_fg, _, _) =
        assert_min_contrast(&mut surface, &lbl_disabled, &win, 0, 3.0, "disabled label");
    assert!(
        relative_luminance_rgb(dis_fg) > relative_luminance_rgb(norm_fg),
        "disabled label fg {dis_fg:?} must be visibly lighter than enabled label fg {norm_fg:?}"
    );
    assert_min_contrast(
        &mut surface,
        &btn_flat_disabled,
        &win,
        2,
        3.0,
        "disabled flat button",
    );

    // 3. .dim-label (>= 4.5)
    assert_min_contrast(&mut surface, &lbl_dim, &win, 0, 4.5, ".dim-label");

    // 4. Entry placeholder (>= 4.5; inset 4px inside entry bounds to avoid sampling 1px border)
    assert_min_contrast(&mut surface, &entry, &win, 4, 4.5, "Entry placeholder");

    // 5. Rows: hovered (prelight), zebra even, and selected (title & subtitle >= 4.5)
    assert_min_contrast(
        &mut surface,
        &hover_title_lbl,
        &win,
        0,
        4.5,
        "hovered row title",
    );
    assert_min_contrast(
        &mut surface,
        &hover_sub_lbl,
        &win,
        0,
        4.5,
        "hovered row subtitle",
    );
    assert_min_contrast(
        &mut surface,
        &even_title_lbl,
        &win,
        0,
        4.5,
        "even row title",
    );
    assert_min_contrast(
        &mut surface,
        &even_sub_lbl,
        &win,
        0,
        4.5,
        "even row subtitle",
    );
    assert_min_contrast(
        &mut surface,
        &sel_title_lbl,
        &win,
        0,
        4.5,
        "selected row title",
    );
    assert_min_contrast(
        &mut surface,
        &sel_sub_lbl,
        &win,
        0,
        4.5,
        "selected row subtitle",
    );

    let sel_rect = widget_bounds_in(&row_selected, &win, 0, 0);
    let sel_bar_px = surface_pixel(
        &mut surface,
        sel_rect.x0 + 1,
        (sel_rect.y0 + sel_rect.y1) / 2,
    );
    let sel_bar_rgb = (sel_bar_px.0, sel_bar_px.1, sel_bar_px.2);
    assert!(
        sel_bar_rgb.2 > sel_bar_rgb.0 + 40 && sel_bar_rgb.0 < 120,
        "selected row left edge {sel_bar_rgb:?} must render @accent inset bar"
    );

    // 6. Chips of all four states (>= 4.5; inset 2px to exclude 1px border)
    assert_min_contrast(&mut surface, &chip_idle, &win, 2, 4.5, "status-idle chip");
    assert_min_contrast(&mut surface, &chip_ready, &win, 2, 4.5, "status-ready chip");
    assert_min_contrast(
        &mut surface,
        &chip_active,
        &win,
        2,
        4.5,
        "status-active chip",
    );
    assert_min_contrast(&mut surface, &chip_error, &win, 2, 4.5, "status-error chip");

    // 7. Secondary button (normal, hovered, flat >= 4.5) and accent icon button
    assert_min_contrast(
        &mut surface,
        &btn_secondary,
        &win,
        2,
        4.5,
        "secondary button",
    );
    assert_min_contrast(
        &mut surface,
        &btn_secondary_hover,
        &win,
        2,
        4.5,
        "secondary button hover",
    );
    assert_min_contrast(
        &mut surface,
        &btn_secondary_flat,
        &win,
        2,
        4.5,
        "secondary flat button",
    );
    let (icon_fg, _, _) =
        assert_min_contrast(&mut surface, &icon_btn, &win, 2, 4.5, "accent icon button");
    assert!(
        icon_fg.2 > icon_fg.0 + 40,
        "icon button glyph {icon_fg:?} must be accent blue"
    );

    // 8. Suggested-action button at BOTH vertical gradient stops (>= 4.5)
    let (sug_fg, _, _) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&sug_btn_lbl, &win, 0, 0));
    let sug_rect = widget_bounds_in(&btn_suggested, &win, 2, 2);
    let mid_x = (sug_rect.x0 + sug_rect.x1) / 2;
    let top_stop_px = surface_pixel(&mut surface, mid_x, sug_rect.y0 + 1);
    let bot_stop_px = surface_pixel(&mut surface, mid_x, sug_rect.y1 - 2);
    let top_stop_rgb = (top_stop_px.0, top_stop_px.1, top_stop_px.2);
    let bot_stop_rgb = (bot_stop_px.0, bot_stop_px.1, bot_stop_px.2);
    let cr_top = wcag_contrast_rgb(sug_fg, top_stop_rgb);
    let cr_bot = wcag_contrast_rgb(sug_fg, bot_stop_rgb);
    eprintln!(
        "Stage rendered suggested-action: fg={sug_fg:?}, top_bg={top_stop_rgb:?} ({cr_top:.2}), bot_bg={bot_stop_rgb:?} ({cr_bot:.2})"
    );
    assert!(
        cr_top >= 4.5 && cr_bot >= 4.5,
        "suggested-action contrast must be >= 4.5 at both stops (top={cr_top:.2}, bot={cr_bot:.2})"
    );
    assert!(
        relative_luminance_rgb(top_stop_rgb) > relative_luminance_rgb(bot_stop_rgb),
        "suggested-action top stop {top_stop_rgb:?} must be lighter than bottom stop {bot_stop_rgb:?}"
    );

    // 9. Labels over the bottom of the window gradient (>= 4.5)
    let (_, grad_p_bg, _) = assert_min_contrast(
        &mut surface,
        &lbl_grad_primary,
        &win,
        0,
        4.5,
        "bottom gradient primary label",
    );
    let (_, grad_m_bg, _) = assert_min_contrast(
        &mut surface,
        &lbl_grad_muted,
        &win,
        0,
        4.5,
        "bottom gradient muted label",
    );
    assert!(
        grad_p_bg.2 > grad_p_bg.0 + 10 && grad_m_bg.2 > grad_m_bg.0 + 10,
        "bottom labels must sit on tinted azure window gradient (got {grad_p_bg:?}, {grad_m_bg:?})"
    );

    // 10. Header text at its lightest (right) point (>= 4.5)
    let (_, hdr_light_bg, _) = assert_min_contrast(
        &mut surface,
        &header_right_lbl,
        &win,
        0,
        4.5,
        "header lightest end text",
    );
    assert!(
        hdr_light_bg.2 > 115,
        "header right end bg {hdr_light_bg:?} must show lightened gradient end"
    );

    // 11. Toast label & button label contrast (>= 4.5) + rounded pill corner verification
    let (_, tl_bg, _) =
        assert_min_contrast(&mut surface, &toast_label, &win, 0, 4.5, "Toast label");
    assert_min_contrast(&mut surface, &toast_btn_label, &win, 0, 4.5, "Toast button");

    let toast_outer = widget_bounds_in(&toast_widget, &win, 0, 0);
    let corner_px = surface_pixel(&mut surface, toast_outer.x0 + 1, toast_outer.y0 + 1);
    let corner_rgb = (corner_px.0, corner_px.1, corner_px.2);
    let header_rgb = (
        (ui::tokens::Palette::STAGE.header.rgb().0 * 255.0).round() as u8,
        (ui::tokens::Palette::STAGE.header.rgb().1 * 255.0).round() as u8,
        (ui::tokens::Palette::STAGE.header.rgb().2 * 255.0).round() as u8,
    );
    eprintln!(
        "Stage toast outer top-left corner pixel={corner_rgb:?}, interior bg={tl_bg:?}, header={header_rgb:?}"
    );
    assert!(
        relative_luminance_rgb(corner_rgb) > relative_luminance_rgb(header_rgb) + 0.3,
        "Toast top-left corner {corner_rgb:?} must show rounded pill transparency over light background, not header navy {header_rgb:?}"
    );

    win.close();
    gtk4::style_context_remove_provider_for_display(&display, &provider);
    apply_skin(&provider, ui::tokens::Palette::DEFAULT);
}
