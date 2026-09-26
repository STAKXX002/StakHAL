use std::rc::Rc;
use gtk4::cairo;
use gtk4::prelude::*;
use crate::state::AppState;
use crate::ui::nucleo_pinout::coords;
use crate::ui::tokens;

thread_local! {
    static BOARD_SURFACE: std::cell::OnceCell<cairo::ImageSurface> = const { std::cell::OnceCell::new() };
}

pub fn with_board_surface<R>(f: impl FnOnce(&cairo::ImageSurface) -> R) -> R {
    BOARD_SURFACE.with(|cell| {
        let surface = cell.get_or_init(|| {
            let png_bytes = include_bytes!("../../assets/nucleo_f446re_board.png");
            let mut cursor = std::io::Cursor::new(png_bytes);
            cairo::ImageSurface::create_from_png(&mut cursor)
                .expect("Failed to load nucleo_f446re_board.png")
        });
        f(surface)
    })
}

/// Deterministic module-to-color assignment.
/// Modules present in `project_modules` are sorted alphabetically and assigned
/// successive slots from `MODULE_PALETTE`. If a module is not found,
/// falls back to a stable FNV-1a hash modulo palette length.
/// Pins without any module return `tokens::color::TEXT_MUTED`.
pub fn get_module_color(
    module: Option<&str>,
    project_modules: &[String],
) -> (f64, f64, f64) {
    let module = match module {
        Some(m) if !m.is_empty() => m,
        _ => return tokens::color::TEXT_MUTED,
    };

    let mut sorted: Vec<&str> = project_modules.iter().map(|s| s.as_str()).collect();
    sorted.sort();
    sorted.dedup();

    if let Some(pos) = sorted.iter().position(|&m| m == module) {
        return tokens::color::MODULE_PALETTE[pos % tokens::color::MODULE_PALETTE.len()];
    }

    // Stable FNV-1a hash fallback
    let mut hash: u32 = 2166136261;
    for b in module.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    tokens::color::MODULE_PALETTE[(hash as usize) % tokens::color::MODULE_PALETTE.len()]
}

/// Convert module color tuple to hex string
pub fn get_module_color_hex(
    module: Option<&str>,
    project_modules: &[String],
) -> String {
    let (r, g, b) = get_module_color(module, project_modules);
    format!("#{:02x}{:02x}{:02x}", (r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Draws the bird's-eye view of the board with module-colored dots for active pins.
pub fn draw_board_glance_canvas(
    _area: &gtk4::DrawingArea,
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &Rc<std::cell::RefCell<AppState>>,
) {
    let state_borrow = state.borrow();
    draw_board_glance(cr, width, height, &state_borrow);
}

pub fn draw_board_glance(
    cr: &cairo::Context,
    width: f64,
    height: f64,
    state: &AppState,
) {
    // 1. Canvas background
    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    let _ = cr.paint();

    with_board_surface(|surface| {
        // Query actual pixel dimensions from disk
        let surface_w = surface.width() as f64;
        let surface_h = surface.height() as f64;
        if surface_w <= 0.0 || surface_h <= 0.0 {
            return;
        }

        let pad = 8.0;
        let avail_w = (width - 2.0 * pad).max(10.0);
        let avail_h = (height - 2.0 * pad).max(10.0);
        let board_aspect = surface_w / surface_h;

        let (board_w, board_h) = if avail_w / avail_h > board_aspect {
            (avail_h * board_aspect, avail_h)
        } else {
            (avail_w, avail_w / board_aspect)
        };

        let board_x = (width - board_w) / 2.0;
        let board_y = (height - board_h) / 2.0;

        // Render board graphic fitted to viewport
        let _ = cr.save();
        cr.rectangle(board_x, board_y, board_w, board_h);
        let _ = cr.clip();
        cr.translate(board_x, board_y);
        cr.scale(board_w / surface_w, board_h / surface_h);
        let _ = cr.set_source_surface(surface, 0.0, 0.0);
        let _ = cr.paint();
        let _ = cr.restore();

        // 2. Fetch active pins in loaded project
        let proj_guard = state.project.borrow();
        let project = match &proj_guard.loaded_project {
            Some(p) => p,
            None => {
                // Dimmed placeholder text when no project loaded
                cr.select_font_face(tokens::font::CAIRO_MONO, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                cr.set_font_size(11.0);
                cr.set_source_rgb(tokens::color::TEXT_MUTED.0, tokens::color::TEXT_MUTED.1, tokens::color::TEXT_MUTED.2);
                let msg = "[ No project loaded ]";
                if let Ok(ext) = cr.text_extents(msg) {
                    let _ = cr.move_to((width - ext.width()) / 2.0, height - 12.0);
                    let _ = cr.show_text(msg);
                }
                return;
            }
        };

        // 3. Draw active pin dots
        for pin_cfg in &project.pins {
            let primary_mod = pin_cfg.modules.first().map(|s| s.as_str());
            let (dot_r, dot_g, dot_b) = get_module_color(primary_mod, &project.modules);

            if let Some(loc) = stakhal_core::nucleo_pinout::lookup_pin(&pin_cfg.pin) {
                // Morpho connector
                if let Some((conn, pin_num)) = loc.morpho {
                    if let Some(coord) = coords::get_pin_coord(conn, pin_num) {
                        let px = board_x + coord.norm_x * board_w;
                        let py = board_y + coord.norm_y * board_h;
                        draw_pin_dot(cr, px, py, board_w, dot_r, dot_g, dot_b);
                    }
                }
                // Arduino connector
                if let Some((conn, pin_num, _)) = loc.arduino {
                    if let Some(coord) = coords::get_pin_coord(conn, pin_num) {
                        let px = board_x + coord.norm_x * board_w;
                        let py = board_y + coord.norm_y * board_h;
                        draw_pin_dot(cr, px, py, board_w, dot_r, dot_g, dot_b);
                    }
                }
            }
        }
    });
}

fn draw_pin_dot(cr: &cairo::Context, px: f64, py: f64, board_w: f64, r: f64, g: f64, b: f64) {
    let radius = (board_w * 0.009).clamp(3.5, 6.0);

    // Dark contrasting rim
    cr.arc(px, py, radius + 1.2, 0.0, 2.0 * std::f64::consts::PI);
    cr.set_source_rgb(
        tokens::color::BG_VOID.0,
        tokens::color::BG_VOID.1,
        tokens::color::BG_VOID.2,
    );
    let _ = cr.fill();

    // Solid module color circle
    cr.arc(px, py, radius, 0.0, 2.0 * std::f64::consts::PI);
    cr.set_source_rgb(r, g, b);
    let _ = cr.fill();
}

/// Updates the required module-color legend box beneath/above the board glance.
pub fn update_board_glance_legend(
    box_legend: &gtk4::Box,
    project: Option<&stakhal_core::ir::schema::Project>,
) {
    while let Some(child) = box_legend.first_child() {
        box_legend.remove(&child);
    }

    let project = match project {
        Some(p) => p,
        None => return,
    };

    let mut modules: Vec<String> = project.modules.clone();
    modules.sort();
    modules.dedup();

    let has_unassigned = project.pins.iter().any(|p| p.modules.is_empty());

    for m in &modules {
        let hex = get_module_color_hex(Some(m), &project.modules);
        let chip = create_legend_chip(m, &hex);
        box_legend.append(&chip);
    }

    if has_unassigned {
        let chip = create_legend_chip("unassigned", tokens::color::TEXT_MUTED_HEX);
        box_legend.append(&chip);
    }
}

fn create_legend_chip(label_text: &str, color_hex: &str) -> gtk4::Box {
    let chip = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(4)
        .margin_start(4)
        .margin_end(4)
        .margin_top(1)
        .margin_bottom(1)
        .build();

    let dot = gtk4::Label::builder()
        .label("●")
        .use_markup(true)
        .build();
    dot.set_markup(&format!("<span foreground=\"{}\" font_desc=\"11\">●</span>", color_hex));

    let lbl = gtk4::Label::builder()
        .label(label_text)
        .css_classes(vec!["data-mono".to_string(), "caption".to_string()])
        .build();

    chip.append(&dot);
    chip.append(&lbl);
    chip
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_surface_dimensions() {
        with_board_surface(|surface| {
            assert_eq!(surface.width(), 1400);
            assert_eq!(surface.height(), 1650);
        });
    }

    #[test]
    fn test_module_color_deterministic_sorting() {
        let modules = vec![
            "alignment".to_string(),
            "gripper".to_string(),
            "motion".to_string(),
        ];

        let c_align = get_module_color(Some("alignment"), &modules);
        let c_grip = get_module_color(Some("gripper"), &modules);
        let c_motion = get_module_color(Some("motion"), &modules);

        assert_eq!(c_align, tokens::color::MODULE_PALETTE[0]);
        assert_eq!(c_grip, tokens::color::MODULE_PALETTE[1]);
        assert_eq!(c_motion, tokens::color::MODULE_PALETTE[2]);

        // Unassigned pins get TEXT_MUTED
        let c_none = get_module_color(None, &modules);
        assert_eq!(c_none, tokens::color::TEXT_MUTED);
    }

    #[test]
    fn test_draw_board_glance_smoke() {
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 400, 500).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        let state = AppState::default();

        draw_board_glance(&cr, 400.0, 500.0, &state);
        surface.flush();
    }

    #[test]
    fn test_board_glance_aa_ns_stm_port() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let ioc_path = fixture_dir.join("aa_ns_stm_port.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load aa_ns_stm_port");

        let state = AppState::default();
        state.project.borrow_mut().loaded_project = Some(project);

        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 500, 600).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();

        draw_board_glance(&cr, 500.0, 600.0, &state);
        surface.flush();

        let out_path = "/home/stakxx002/.gemini/antigravity-ide/brain/e21edbbd-844e-44ef-9dfa-1af3c8e3a19b/dashboard_board_glance_verification.png";
        let mut file = std::fs::File::create(out_path).expect("Failed to create output PNG");
        surface.write_to_png(&mut file).expect("Failed to write PNG");
    }
}
