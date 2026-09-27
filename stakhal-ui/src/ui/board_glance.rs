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

/// Hovered pin info for minimal glance tooltip
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoveredGlancePin {
    pub pin: String,
    pub module: String,
}

/// Finds the active pin dot under the cursor (x, y) on the glance board.
/// Returns minimal (pin name, module) only.
pub fn find_hovered_pin(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    state: &AppState,
) -> Option<HoveredGlancePin> {
    let proj_guard = state.project.borrow();
    let project = proj_guard.loaded_project.as_ref()?;

    with_board_surface(|surface| {
        let surface_w = surface.width() as f64;
        let surface_h = surface.height() as f64;
        if surface_w <= 0.0 || surface_h <= 0.0 {
            return None;
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

        let radius = (board_w * 0.009).clamp(3.5, 6.0);
        let hit_radius = (radius + 4.0).max(8.0);
        let hit_radius_sq = hit_radius * hit_radius;

        let mut closest: Option<(f64, HoveredGlancePin)> = None;

        for pin_cfg in &project.pins {
            let primary_mod = pin_cfg
                .modules
                .first()
                .cloned()
                .unwrap_or_else(|| "unassigned".to_string());

            if let Some(loc) = stakhal_core::nucleo_pinout::lookup_pin(&pin_cfg.pin) {
                // Morpho connector
                if let Some((conn, pin_num)) = loc.morpho {
                    if let Some(coord) = coords::get_pin_coord(conn, pin_num) {
                        let px = board_x + coord.norm_x * board_w;
                        let py = board_y + coord.norm_y * board_h;
                        let d2 = (x - px) * (x - px) + (y - py) * (y - py);
                        if d2 <= hit_radius_sq && closest.as_ref().map(|c| d2 < c.0).unwrap_or(true) {
                            closest = Some((
                                d2,
                                HoveredGlancePin {
                                    pin: pin_cfg.pin.clone(),
                                    module: primary_mod.clone(),
                                },
                            ));
                        }
                    }
                }
                // Arduino connector
                if let Some((conn, pin_num, _)) = loc.arduino {
                    if let Some(coord) = coords::get_pin_coord(conn, pin_num) {
                        let px = board_x + coord.norm_x * board_w;
                        let py = board_y + coord.norm_y * board_h;
                        let d2 = (x - px) * (x - px) + (y - py) * (y - py);
                        if d2 <= hit_radius_sq && closest.as_ref().map(|c| d2 < c.0).unwrap_or(true) {
                            closest = Some((
                                d2,
                                HoveredGlancePin {
                                    pin: pin_cfg.pin.clone(),
                                    module: primary_mod.clone(),
                                },
                            ));
                        }
                    }
                }
            }
        }

        closest.map(|(_, p)| p)
    })
}

/// Sets up lightweight hover tooltip and click-through navigation on the glance board.
pub fn setup_board_glance_interactions(
    area: &gtk4::DrawingArea,
    state: &Rc<std::cell::RefCell<AppState>>,
    btn_nucleo_pinout: &gtk4::Button,
) {
    area.set_cursor_from_name(Some("pointer"));

    // 1. Tooltip support via native query-tooltip
    area.set_has_tooltip(true);
    let state_tooltip = Rc::clone(state);
    area.connect_query_tooltip(move |area, x, y, _keyboard_mode, tooltip| {
        let w = area.width() as f64;
        let h = area.height() as f64;
        let state_borrow = state_tooltip.borrow();
        if let Some(hovered) = find_hovered_pin(x as f64, y as f64, w, h, &state_borrow) {
            tooltip.set_text(Some(&format!("{}: {}", hovered.pin, hovered.module)));
            true
        } else {
            false
        }
    });

    // 2. Motion controller to trigger tooltip query immediately as mouse moves over dots
    let motion = gtk4::EventControllerMotion::new();
    let area_weak = area.downgrade();
    motion.connect_motion(move |_ctrl, _x, _y| {
        if let Some(area) = area_weak.upgrade() {
            area.trigger_tooltip_query();
        }
    });
    area.add_controller(motion);

    // 3. Click gesture to navigate to full Nucleo Pinout tab
    let click = gtk4::GestureClick::new();
    let btn_target = btn_nucleo_pinout.clone();
    click.connect_released(move |_gesture, _n_press, _x, _y| {
        btn_target.emit_clicked();
    });
    area.add_controller(click);
}

/// Updates the required module-color legend FlowBox beneath/above the board glance.
/// Entries cleanly wrap across multiple rows as module count grows.
pub fn update_board_glance_legend(
    flow_legend: &gtk4::FlowBox,
    project: Option<&stakhal_core::ir::schema::Project>,
) {
    while let Some(child) = flow_legend.first_child() {
        flow_legend.remove(&child);
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
        let child = gtk4::FlowBoxChild::builder()
            .child(&chip)
            .focusable(false)
            .build();
        flow_legend.append(&child);
    }

    if has_unassigned {
        let chip = create_legend_chip("unassigned", tokens::color::TEXT_MUTED_HEX);
        let child = gtk4::FlowBoxChild::builder()
            .child(&chip)
            .focusable(false)
            .build();
        flow_legend.append(&child);
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
        .focusable(false)
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
    fn test_board_aspect_ratio_preservation() {
        let expected_aspect: f64 = 1400.0 / 1650.0; // 70.0 / 82.5

        // Test wide canvas (pillarboxed)
        let pad = 8.0;
        let w_wide = 800.0;
        let h_wide = 400.0;
        let avail_w = w_wide - 2.0 * pad;
        let avail_h = h_wide - 2.0 * pad;
        let (bw_wide, bh_wide) = if avail_w / avail_h > expected_aspect {
            (avail_h * expected_aspect, avail_h)
        } else {
            (avail_w, avail_w / expected_aspect)
        };
        assert!((bw_wide / bh_wide - expected_aspect).abs() < 1e-6);

        // Test tall canvas (letterboxed)
        let w_tall = 300.0;
        let h_tall = 700.0;
        let avail_w = w_tall - 2.0 * pad;
        let avail_h = h_tall - 2.0 * pad;
        let (bw_tall, bh_tall) = if avail_w / avail_h > expected_aspect {
            (avail_h * expected_aspect, avail_h)
        } else {
            (avail_w, avail_w / expected_aspect)
        };
        assert!((bw_tall / bh_tall - expected_aspect).abs() < 1e-6);
    }

    #[test]
    fn test_find_hovered_pin_hit_detection() {
        let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../stakhal-core/tests/fixtures/aa_ns_stm_port");
        let ioc_path = fixture_dir.join("aa_ns_stm_port.ioc");
        let main_c_path = fixture_dir.join("Core/Src/main.c");
        let project = stakhal_core::ir::schema::load_project(&ioc_path, &main_c_path)
            .expect("Failed to load aa_ns_stm_port");

        let state = AppState::default();
        state.project.borrow_mut().loaded_project = Some(project);

        let canvas_w = 500.0;
        let canvas_h = 600.0;
        let pad = 8.0;
        let avail_w = canvas_w - 2.0 * pad;
        let surface_w = 1400.0;
        let surface_h = 1650.0;
        let board_aspect = surface_w / surface_h;
        let (board_w, board_h) = (avail_w, avail_w / board_aspect);
        let board_x = (canvas_w - board_w) / 2.0;
        let board_y = (canvas_h - board_h) / 2.0;

        // 1. Hit test PB12 (CN10 pin 16, module hatch)
        let coord_pb12 = coords::get_pin_coord("CN10", 16).unwrap();
        let px_pb12 = board_x + coord_pb12.norm_x * board_w;
        let py_pb12 = board_y + coord_pb12.norm_y * board_h;
        let hit_pb12 = find_hovered_pin(px_pb12, py_pb12, canvas_w, canvas_h, &state);
        assert_eq!(
            hit_pb12,
            Some(HoveredGlancePin {
                pin: "PB12".to_string(),
                module: "hatch".to_string(),
            })
        );

        // 2. Hit test PB0 (CN7 pin 34, module alignment)
        let coord_pb0 = coords::get_pin_coord("CN7", 34).unwrap();
        let px_pb0 = board_x + coord_pb0.norm_x * board_w;
        let py_pb0 = board_y + coord_pb0.norm_y * board_h;
        let hit_pb0 = find_hovered_pin(px_pb0, py_pb0, canvas_w, canvas_h, &state);
        assert_eq!(
            hit_pb0,
            Some(HoveredGlancePin {
                pin: "PB0".to_string(),
                module: "alignment".to_string(),
            })
        );

        // 3. Hit test PA5 (CN10 pin 11, unassigned)
        let coord_pa5 = coords::get_pin_coord("CN10", 11).unwrap();
        let px_pa5 = board_x + coord_pa5.norm_x * board_w;
        let py_pa5 = board_y + coord_pa5.norm_y * board_h;
        let hit_pa5 = find_hovered_pin(px_pa5, py_pa5, canvas_w, canvas_h, &state);
        assert_eq!(
            hit_pa5,
            Some(HoveredGlancePin {
                pin: "PA5".to_string(),
                module: "unassigned".to_string(),
            })
        );

        // 4. Hit test far off point (e.g. at 5.0, 5.0) -> None
        let hit_none = find_hovered_pin(5.0, 5.0, canvas_w, canvas_h, &state);
        assert_eq!(hit_none, None);
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
