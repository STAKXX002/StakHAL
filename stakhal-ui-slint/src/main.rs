// StakHAL Slint UI Prototype
// Open-source under MIT License.
// Slint is used under Royalty-Free / Community License terms for non-commercial open-source software.

use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use slint::VecModel;
use stakhal_core::graph::{build_call_graph, compute_graph_bounds, compute_graph_layout};
use stakhal_core::ioc::parse_ioc;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Locate and parse real CubeMX project fixture (.ioc)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let fixture_path = manifest_dir.join("../stakhal-core/tests/fixtures/stm32_03_timers/03_timers.ioc");
    let fallback_path = manifest_dir.join("../stakhal-core/tests/fixtures/stakhal_blink_f446re/stakhal_blink_f446re.ioc");

    let (ioc_path, project_name) = if fixture_path.exists() {
        (fixture_path, "stm32_03_timers")
    } else {
        (fallback_path, "stakhal_blink_f446re")
    };

    println!("[STAKHAL SLINT] Loading project fixture from: {:?}", ioc_path);
    let ioc_project = parse_ioc(&ioc_path).map_err(|e| format!("Failed to parse .ioc file: {}", e))?;
    let edges = build_call_graph(&ioc_project);
    println!("[STAKHAL SLINT] Call graph generated with {} edges.", edges.len());

    // 2. Initialize Slint MainWindow
    let window = MainWindow::new()?;
    window.set_project_name(project_name.into());

    let collapsed_chains = Rc::new(std::cell::RefCell::new(HashSet::<String>::new()));

    // Helper closure to update Slint properties whenever layout recalculates
    let update_ui_model = {
        let window_weak = window.as_weak();
        let edges_clone = edges.clone();
        let collapsed_ref = collapsed_chains.clone();

        move || {
            let Some(win) = window_weak.upgrade() else { return };
            let collapsed = collapsed_ref.borrow();
            let (positions, headers) = compute_graph_layout(&edges_clone, &collapsed);
            let (bw, bh) = compute_graph_bounds(&positions, &headers);

            win.set_canvas_width(bw as f32);
            win.set_canvas_height(bh as f32);

            // Build NodeData vector
            let mut node_list: Vec<NodeData> = Vec::new();

            for (n_id, &(n_x, n_y)) in &positions {
                let n_w = (n_id.len() as f64 * 8.5 + 28.0).max(110.0);
                let n_h = 34.0;
                node_list.push(NodeData {
                    id: n_id.clone().into(),
                    label: n_id.clone().into(),
                    x: n_x as f32,
                    y: n_y as f32,
                    width: n_w as f32,
                    height: n_h as f32,
                    is_header: false,
                    is_collapsed: false,
                    handler_id: "".into(),
                });
            }

            for h in &headers {
                let icon = if h.is_collapsed { "▸" } else { "▾" };
                let label = format!("{} {}", icon, h.label);
                node_list.push(NodeData {
                    id: h.handler_id.clone().into(),
                    label: label.into(),
                    x: h.x as f32,
                    y: h.y as f32,
                    width: h.w as f32,
                    height: h.h as f32,
                    is_header: true,
                    is_collapsed: h.is_collapsed,
                    handler_id: h.handler_id.clone().into(),
                });
            }

            // Build EdgeData vector
            let mut edge_list: Vec<EdgeData> = Vec::new();
            for e in &edges_clone {
                if let (Some(&(fx, fy)), Some(&(tx, ty))) = (positions.get(&e.from), positions.get(&e.to)) {
                    let fw = (e.from.len() as f64 * 8.5 + 28.0).max(110.0);
                    let fh = 34.0;
                    let tw = (e.to.len() as f64 * 8.5 + 28.0).max(110.0);
                    let th = 34.0;

                    let fc = (fx + fw / 2.0, fy + fh / 2.0);
                    let tc = (tx + tw / 2.0, ty + th / 2.0);

                    let (sx, sy) = get_rect_ray_intersection(fx, fy, fw, fh, tc.0, tc.1);
                    let (ex, ey) = get_rect_ray_intersection(tx, ty, tw, th, fc.0, fc.1);

                    let dx = ex - sx;
                    let dy = ey - sy;

                    let (cp1_x, cp1_y, cp2_x, cp2_y) = if dy.abs() >= dx.abs() {
                        let offset_y = (dy.abs() * 0.55).max(40.0);
                        let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                        (sx, sy + offset_y * sign_y, ex, ey - offset_y * sign_y)
                    } else {
                        let offset_x = (dx.abs() * 0.55).max(40.0);
                        let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                        (sx + offset_x * sign_x, sy, ex - offset_x * sign_x, ey)
                    };

                    edge_list.push(EdgeData {
                        x1: sx as f32,
                        y1: sy as f32,
                        x2: ex as f32,
                        y2: ey as f32,
                        cp1_x: cp1_x as f32,
                        cp1_y: cp1_y as f32,
                        cp2_x: cp2_x as f32,
                        cp2_y: cp2_y as f32,
                    });
                }
            }

            for (idx, (e, ge)) in edge_list.iter().zip(&edges_clone).take(3).enumerate() {
                let from_pos = positions.get(&ge.from).cloned().unwrap_or((0.0, 0.0));
                let to_pos = positions.get(&ge.to).cloned().unwrap_or((0.0, 0.0));
                println!(
                    "[STAKHAL DEBUG] Edge {} ({} -> {}): node_from({:.1}, {:.1}), node_to({:.1}, {:.1}) | Path MoveTo ({:.1}, {:.1}) -> CubicTo cp1({:.1}, {:.1}), cp2({:.1}, {:.1}), end({:.1}, {:.1})",
                    idx, ge.from, ge.to, from_pos.0, from_pos.1, to_pos.0, to_pos.1, e.x1, e.y1, e.cp1_x, e.cp1_y, e.cp2_x, e.cp2_y, e.x2, e.y2
                );
            }

            win.set_nodes(Rc::new(VecModel::from(node_list)).into());
            win.set_edges(Rc::new(VecModel::from(edge_list)).into());
        }
    };

    // Initial render layout
    update_ui_model();

    // 3. Connect interactive callback for header click (collapse/expand)
    let collapsed_chains_click = collapsed_chains.clone();
    let update_ui_click = update_ui_model.clone();
    let window_weak_click = window.as_weak();
    let snapshot_path = manifest_dir.join("../artifacts/slint_callgraph_prototype.png");
    let snapshot_path_click = snapshot_path.clone();

    window.on_header_clicked(move |handler_id| {
        let handler_str = handler_id.to_string();
        println!("[STAKHAL SLINT] Header clicked: {}", handler_str);
        {
            let mut set = collapsed_chains_click.borrow_mut();
            if set.contains(&handler_str) {
                set.remove(&handler_str);
            } else {
                set.insert(handler_str);
            }
        }
        update_ui_click();

        if let Some(win) = window_weak_click.upgrade() {
            if let Ok(pixbuf) = win.window().take_snapshot() {
                let w = pixbuf.width();
                let h = pixbuf.height();
                if w > 0 && h > 0 {
                    let b = pixbuf.as_bytes();
                    if let Some(img_buf) = image::RgbaImage::from_raw(w, h, b.to_vec()) {
                        let _ = img_buf.save(&snapshot_path_click);
                        println!("[STAKHAL SLINT] Updated snapshot on click to: {:?}", snapshot_path_click);
                    }
                }
            }
        }
    });

    // Schedule initial snapshot
    let window_weak_timer = window.as_weak();
    let snapshot_path_timer = snapshot_path.clone();
    slint::Timer::single_shot(std::time::Duration::from_millis(200), move || {
        if let Some(win) = window_weak_timer.upgrade() {
            if let Ok(pixbuf) = win.window().take_snapshot() {
                let w = pixbuf.width();
                let h = pixbuf.height();
                if w > 0 && h > 0 {
                    let b = pixbuf.as_bytes();
                    if let Some(img_buf) = image::RgbaImage::from_raw(w, h, b.to_vec()) {
                        let _ = img_buf.save(&snapshot_path_timer);
                        println!("[STAKHAL SLINT] Saved initial prototype snapshot to: {:?}", snapshot_path_timer);
                    }
                }
            }
        }
    });

    println!("[STAKHAL SLINT] Launching Slint prototype window...");
    window.run()?;

    Ok(())
}

fn get_rect_ray_intersection(
    rect_x: f64,
    rect_y: f64,
    rect_w: f64,
    rect_h: f64,
    target_x: f64,
    target_y: f64,
) -> (f64, f64) {
    let cx = rect_x + rect_w / 2.0;
    let cy = rect_y + rect_h / 2.0;

    let dx = target_x - cx;
    let dy = target_y - cy;

    if dx == 0.0 && dy == 0.0 {
        return (cx, cy);
    }

    let scale_x = if dx > 0.0 {
        (rect_w / 2.0) / dx
    } else if dx < 0.0 {
        (-rect_w / 2.0) / dx
    } else {
        f64::INFINITY
    };

    let scale_y = if dy > 0.0 {
        (rect_h / 2.0) / dy
    } else if dy < 0.0 {
        (-rect_h / 2.0) / dy
    } else {
        f64::INFINITY
    };

    let scale = scale_x.min(scale_y);

    (cx + dx * scale, cy + dy * scale)
}
