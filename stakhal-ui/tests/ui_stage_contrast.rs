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

    let card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .css_classes(vec!["card".to_string()])
        .build();

    let inner = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
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

    let list_box = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .build();
    let action_row = ui::main_panel::create_peripheral_row("USART2", Some("Asynchronous"), 6);
    list_box.append(&action_row);

    inner.append(&lbl_normal);
    inner.append(&lbl_disabled);
    inner.append(&btn_flat_disabled);
    inner.append(&lbl_dim);
    inner.append(&entry);
    inner.append(&list_box);

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .vexpand(true)
        .build();
    root_box.append(&card);

    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&root_box));

    let win = adw::ApplicationWindow::builder()
        .default_width(640)
        .default_height(460)
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

    let row_title_lbl = find_descendant(action_row.upcast_ref(), &|w| {
        w.downcast_ref::<gtk4::Label>()
            .is_some_and(|l| l.text() == "USART2")
    })
    .expect("ActionRow must contain title label");

    let row_subtitle_lbl = find_descendant(action_row.upcast_ref(), &|w| {
        w.downcast_ref::<gtk4::Label>()
            .is_some_and(|l| l.text() == "Asynchronous")
    })
    .expect("ActionRow must contain subtitle label");

    let toast_widget = find_descendant(toast_overlay.upcast_ref(), &|w| w.css_name() == "toast")
        .expect("ToastOverlay must contain mapped toast widget");
    assert!(
        toast_widget.is_mapped() && toast_widget.height() > 20 && toast_widget.width() > 80,
        "toast must be mapped and sized cleanly ({}x{})",
        toast_widget.width(),
        toast_widget.height()
    );

    let toast_label = find_descendant(&toast_widget, &|w| {
        w.downcast_ref::<gtk4::Label>()
            .is_some_and(|l| l.text() == "Stage Skin Activated")
    })
    .expect("Toast must contain title label");

    let toast_btn_label = find_descendant(&toast_widget, &|w| {
        w.downcast_ref::<gtk4::Label>()
            .is_some_and(|l| l.text() == "Dismiss")
    })
    .expect("Toast must contain action button label");

    let mut surface = render_widget_to_surface(&win);

    // 1. Normal label (>= 4.5)
    let (norm_fg, norm_bg, norm_cr) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&lbl_normal, &win, 0, 0));
    eprintln!("Stage rendered normal label: fg={norm_fg:?} bg={norm_bg:?} contrast={norm_cr:.2}");
    assert!(
        norm_cr >= 4.5,
        "normal label contrast {norm_cr:.2} < 4.5 (fg={norm_fg:?}, bg={norm_bg:?})"
    );

    // 2. Disabled label and disabled flat button (>= 3.0 AND visibly lighter than enabled label)
    let (dis_fg, dis_bg, dis_cr) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&lbl_disabled, &win, 0, 0));
    eprintln!("Stage rendered disabled label: fg={dis_fg:?} bg={dis_bg:?} contrast={dis_cr:.2}");
    assert!(
        dis_cr >= 3.0,
        "disabled label contrast {dis_cr:.2} < 3.0 (fg={dis_fg:?}, bg={dis_bg:?})"
    );
    assert!(
        relative_luminance_rgb(dis_fg) > relative_luminance_rgb(norm_fg),
        "disabled label fg {dis_fg:?} must be visibly lighter than enabled label fg {norm_fg:?}"
    );

    let (btn_dis_fg, btn_dis_bg, btn_dis_cr) = sample_fg_and_bg_in_rect(
        &mut surface,
        widget_bounds_in(&btn_flat_disabled, &win, 2, 2),
    );
    eprintln!(
        "Stage rendered disabled flat button: fg={btn_dis_fg:?} bg={btn_dis_bg:?} contrast={btn_dis_cr:.2}"
    );
    assert!(
        btn_dis_cr >= 3.0,
        "disabled flat button contrast {btn_dis_cr:.2} < 3.0 (fg={btn_dis_fg:?}, bg={btn_dis_bg:?})"
    );

    // 3. .dim-label (>= 4.5)
    let (dim_fg, dim_bg, dim_cr) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&lbl_dim, &win, 0, 0));
    eprintln!("Stage rendered .dim-label: fg={dim_fg:?} bg={dim_bg:?} contrast={dim_cr:.2}");
    assert!(
        dim_cr >= 4.5,
        ".dim-label contrast {dim_cr:.2} < 4.5 (fg={dim_fg:?}, bg={dim_bg:?})"
    );

    // 4. AdwActionRow title and subtitle (>= 4.5)
    let (art_fg, art_bg, art_cr) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&row_title_lbl, &win, 0, 0));
    eprintln!("Stage rendered ActionRow title: fg={art_fg:?} bg={art_bg:?} contrast={art_cr:.2}");
    assert!(
        art_cr >= 4.5,
        "ActionRow title contrast {art_cr:.2} < 4.5 (fg={art_fg:?}, bg={art_bg:?})"
    );

    let (ars_fg, ars_bg, ars_cr) = sample_fg_and_bg_in_rect(
        &mut surface,
        widget_bounds_in(&row_subtitle_lbl, &win, 0, 0),
    );
    eprintln!(
        "Stage rendered ActionRow subtitle: fg={ars_fg:?} bg={ars_bg:?} contrast={ars_cr:.2}"
    );
    assert!(
        ars_cr >= 4.5,
        "ActionRow subtitle contrast {ars_cr:.2} < 4.5 (fg={ars_fg:?}, bg={ars_bg:?})"
    );

    // 5. Entry placeholder (>= 4.5; inset 4px inside entry bounds to avoid sampling 1px border)
    let entry_inner_rect = widget_bounds_in(&entry, &win, 4, 4);
    let (ent_fg, ent_bg, ent_cr) = sample_fg_and_bg_in_rect(&mut surface, entry_inner_rect);
    eprintln!("Stage rendered Entry placeholder: fg={ent_fg:?} bg={ent_bg:?} contrast={ent_cr:.2}");
    assert!(
        ent_cr >= 4.5,
        "Entry placeholder contrast {ent_cr:.2} < 4.5 (fg={ent_fg:?}, bg={ent_bg:?})"
    );

    // 6. Toast label & button label contrast (>= 4.5) + rounded pill corner verification
    let (tl_fg, tl_bg, tl_cr) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&toast_label, &win, 0, 0));
    eprintln!(
        "Stage rendered Toast label: classes={:?} fg={tl_fg:?} bg={tl_bg:?} contrast={tl_cr:.2}",
        toast_label.css_classes()
    );
    assert!(
        tl_cr >= 4.5,
        "Toast label contrast {tl_cr:.2} < 4.5 (fg={tl_fg:?}, bg={tl_bg:?})"
    );

    let (tb_fg, tb_bg, tb_cr) =
        sample_fg_and_bg_in_rect(&mut surface, widget_bounds_in(&toast_btn_label, &win, 0, 0));
    eprintln!("Stage rendered Toast button: fg={tb_fg:?} bg={tb_bg:?} contrast={tb_cr:.2}");
    assert!(
        tb_cr >= 4.5,
        "Toast button contrast {tb_cr:.2} < 4.5 (fg={tb_fg:?}, bg={tb_bg:?})"
    );

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
