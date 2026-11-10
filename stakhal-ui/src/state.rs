use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_core::ir::schema::Project;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// Domain cell: project files and loaded core model
#[derive(Default, Clone)]
pub struct ProjectState {
    pub project_dir: Option<PathBuf>,
    pub discovered_ioc: Option<PathBuf>,
    pub discovered_main_c: Option<PathBuf>,
    pub loaded_project: Option<Project>,
}

/// Domain cell: compilation pipeline, flashing progress, and build traceability
#[derive(Default, Clone)]
pub struct BuildTraceState {
    pub build_in_progress: bool,
    pub has_makefile: bool,
    pub has_build_system: bool,
    pub detected_build_system: Option<crate::toolchain::builder::BuildSystem>,
    pub selected_probe: Option<String>,
    pub is_traceability_enabled: bool,
    pub captured_build_hash: Option<String>,
    pub is_captured_hash_dirty: bool,
    pub traceability_verify_start: Option<std::time::Instant>,
    pub flash_verify_start: Option<std::time::Instant>,
    pub traceability_was_matched: bool,
}

/// Domain cell: serial communication session, history, and configuration
pub struct SerialState {
    pub detected_console_uart: Option<stakhal_core::source::ConsoleUartInfo>,
    pub available_serial_ports: Vec<crate::toolchain::serial::SerialPortInfo>,
    pub selected_serial_port: Option<String>,
    pub selected_serial_baud: u32,
    pub is_serial_connected: bool,
    pub serial_session: Option<crate::toolchain::serial::ActiveSerialSession>,
    pub serial_command_history: Vec<String>,
    pub serial_history_index: Option<usize>,
}

impl Default for SerialState {
    fn default() -> Self {
        Self {
            detected_console_uart: None,
            available_serial_ports: Vec::new(),
            selected_serial_port: None,
            selected_serial_baud: 115200,
            is_serial_connected: false,
            serial_session: None,
            serial_command_history: Vec::new(),
            serial_history_index: None,
        }
    }
}

/// Domain cell: interactive state diagram graph and nucleo pinout canvas view state
#[derive(Clone)]
pub struct DiagramCanvasState {
    pub selected_state_machine: usize,
    pub state_diagram_layout: Option<stakhal_core::graph::StateMachineLayout>,
    pub diagram_zoom: f64,
    pub diagram_pan_x: f64,
    pub diagram_pan_y: f64,
    pub diagram_mouse_pos: Option<(f64, f64)>,
    pub diagram_needs_fit: bool,
    pub selected_state_node: Option<String>,
    pub live_state_node: Option<String>,
    pub hovered_state_node: Option<String>,
    pub state_node_positions: std::collections::HashMap<String, (f64, f64)>,
    pub diagram_bounds: (i32, i32),
    pub drag_start_click_pos: (f64, f64),
    pub drag_start_pan_pos: (f64, f64),
    pub hovered_pinout_pin: Option<(String, u8)>,
    pub hovered_pinout_mouse: Option<(f64, f64)>,
    pub selected_pinout_module: Option<String>,
    pub node_flash_animation: Option<(String, std::time::Instant)>,
    pub edge_reveal_animation: Option<(usize, std::time::Instant)>,
    pub edge_pulse_animation: Option<(String, String, std::time::Instant)>,
    pub session_revealed_machines: std::collections::HashSet<usize>,
    pub node_hover_amounts: std::collections::HashMap<String, f64>,
    pub node_hover_last_step: Option<std::time::Instant>,
    pub diagram_tick_active: bool,
    pub pin_hover_amounts: std::collections::HashMap<(String, u8), f64>,
    pub pin_hover_last_step: Option<std::time::Instant>,
    pub pinout_tick_active: bool,
}

impl Default for DiagramCanvasState {
    fn default() -> Self {
        Self {
            selected_state_machine: 0,
            state_diagram_layout: None,
            diagram_zoom: 0.95,
            diagram_pan_x: 40.0,
            diagram_pan_y: 40.0,
            diagram_mouse_pos: None,
            diagram_needs_fit: true,
            selected_state_node: None,
            live_state_node: None,
            hovered_state_node: None,
            state_node_positions: std::collections::HashMap::new(),
            diagram_bounds: (1200, 800),
            drag_start_click_pos: (0.0, 0.0),
            drag_start_pan_pos: (40.0, 40.0),
            hovered_pinout_pin: None,
            hovered_pinout_mouse: None,
            selected_pinout_module: None,
            node_flash_animation: None,
            edge_reveal_animation: None,
            edge_pulse_animation: None,
            session_revealed_machines: std::collections::HashSet::new(),
            node_hover_amounts: std::collections::HashMap::new(),
            node_hover_last_step: None,
            diagram_tick_active: false,
            pin_hover_amounts: std::collections::HashMap::new(),
            pin_hover_last_step: None,
            pinout_tick_active: false,
        }
    }
}

fn step_hover_map<K: Eq + std::hash::Hash + Clone>(
    amounts: &mut std::collections::HashMap<K, f64>,
    active_key: Option<&K>,
    dt_seconds: f64,
    is_stage_animated: bool,
) -> (bool, bool) {
    if !is_stage_animated {
        let changed = !amounts.is_empty();
        amounts.clear();
        return (changed, false);
    }

    let tau = crate::ui::tokens::motion::HOVER_EASE_TAU_SECONDS;
    let eps = crate::ui::tokens::motion::HOVER_EASE_EPSILON;
    let mut changed = false;
    let mut still_pending = false;
    let mut to_remove: Vec<K> = Vec::new();

    for (k, val) in amounts.iter_mut() {
        let is_target_active = active_key.is_some_and(|ak| ak == k);
        let target = if is_target_active { 1.0 } else { 0.0 };
        let next = crate::ui::tokens::motion::ease_toward(*val, target, dt_seconds, tau);
        if (next - target).abs() <= eps {
            if is_target_active {
                if (*val - 1.0).abs() > 1e-12 {
                    *val = 1.0;
                    changed = true;
                }
            } else {
                to_remove.push(k.clone());
                changed = true;
            }
        } else {
            if (next - *val).abs() > 1e-12 {
                changed = true;
            }
            *val = next;
            still_pending = true;
        }
    }

    for k in to_remove {
        amounts.remove(&k);
    }

    (changed, still_pending)
}

impl DiagramCanvasState {
    pub fn has_unsettled_node_hover(&self) -> bool {
        let eps = crate::ui::tokens::motion::HOVER_EASE_EPSILON;
        self.node_hover_amounts.iter().any(|(id, val)| {
            let target = if self.hovered_state_node.as_deref() == Some(id.as_str()) {
                1.0
            } else {
                0.0
            };
            (val - target).abs() > eps
        })
    }

    pub fn has_unsettled_pin_hover(&self) -> bool {
        let eps = crate::ui::tokens::motion::HOVER_EASE_EPSILON;
        self.pin_hover_amounts.iter().any(|(pin, val)| {
            let target = if self.hovered_pinout_pin.as_ref() == Some(pin) {
                1.0
            } else {
                0.0
            };
            (val - target).abs() > eps
        })
    }

    pub fn has_active_diagram_animations(&self) -> bool {
        self.node_flash_animation.is_some()
            || self.edge_reveal_animation.is_some()
            || self.edge_pulse_animation.is_some()
            || self.has_unsettled_node_hover()
    }

    pub fn set_diagram_hovered_node(
        &mut self,
        hovered: Option<String>,
        is_stage_animated: bool,
        now: std::time::Instant,
    ) -> bool {
        let changed = self.hovered_state_node != hovered;
        self.hovered_state_node = hovered.clone();
        if is_stage_animated {
            if let Some(id) = hovered {
                self.node_hover_amounts.entry(id).or_insert(0.0);
            }
            let unsettled = self.has_unsettled_node_hover();
            if unsettled && self.node_hover_last_step.is_none() {
                self.node_hover_last_step = Some(now);
            }
            changed || unsettled
        } else {
            let had = !self.node_hover_amounts.is_empty();
            self.node_hover_amounts.clear();
            self.node_hover_last_step = None;
            changed || had
        }
    }

    pub fn set_pinout_hovered_pin(
        &mut self,
        hovered: Option<(String, u8)>,
        is_stage_animated: bool,
        now: std::time::Instant,
    ) -> bool {
        let changed = self.hovered_pinout_pin != hovered;
        self.hovered_pinout_pin = hovered.clone();
        if is_stage_animated {
            if let Some(pin) = hovered {
                self.pin_hover_amounts.entry(pin).or_insert(0.0);
            }
            let unsettled = self.has_unsettled_pin_hover();
            if unsettled && self.pin_hover_last_step.is_none() {
                self.pin_hover_last_step = Some(now);
            }
            changed || unsettled
        } else {
            let had = !self.pin_hover_amounts.is_empty();
            self.pin_hover_amounts.clear();
            self.pin_hover_last_step = None;
            changed || had
        }
    }

    pub fn step_node_hover_amounts(
        &mut self,
        dt_seconds: f64,
        is_stage_animated: bool,
    ) -> (bool, bool) {
        let active = self.hovered_state_node.clone();
        let res = step_hover_map(
            &mut self.node_hover_amounts,
            active.as_ref(),
            dt_seconds,
            is_stage_animated,
        );
        if !res.1 {
            self.node_hover_last_step = None;
        }
        res
    }

    pub fn step_pin_hover_amounts(
        &mut self,
        dt_seconds: f64,
        is_stage_animated: bool,
    ) -> (bool, bool) {
        let active = self.hovered_pinout_pin.clone();
        let res = step_hover_map(
            &mut self.pin_hover_amounts,
            active.as_ref(),
            dt_seconds,
            is_stage_animated,
        );
        if !res.1 {
            self.pin_hover_last_step = None;
        }
        res
    }

    pub fn step_diagram_tick(
        &mut self,
        is_stage: bool,
        animations_enabled: bool,
        now: std::time::Instant,
    ) -> (bool, bool) {
        let mut needs_redraw = false;
        let mut anim_pending = false;

        if let Some((_, start)) = self.node_flash_animation {
            let elapsed = now.saturating_duration_since(start).as_millis() as u64;
            if elapsed >= crate::ui::tokens::motion::DURATION_SHORT_MS || !animations_enabled {
                self.node_flash_animation = None;
            } else {
                anim_pending = true;
            }
            needs_redraw = true;
        }

        if let Some((_, start)) = self.edge_reveal_animation {
            let elapsed = now.saturating_duration_since(start).as_millis() as u64;
            if elapsed >= crate::ui::tokens::motion::DURATION_MEDIUM_MS || !animations_enabled {
                self.edge_reveal_animation = None;
            } else {
                anim_pending = true;
            }
            needs_redraw = true;
        }

        if let Some((_, _, start)) = self.edge_pulse_animation {
            let elapsed = now.saturating_duration_since(start).as_millis() as u64;
            let dur = crate::ui::tokens::motion::DURATION_MEDIUM_MS as f64;
            let progress = (elapsed as f64 / dur).clamp(0.0, 1.0);
            if progress >= 1.0
                || elapsed >= crate::ui::tokens::motion::DURATION_MEDIUM_MS
                || !animations_enabled
            {
                self.edge_pulse_animation = None;
            } else {
                anim_pending = true;
            }
            needs_redraw = true;
        }

        let dt = match self.node_hover_last_step.replace(now) {
            Some(prev) => now.saturating_duration_since(prev).as_secs_f64(),
            None => 0.0,
        };
        let (hover_changed, hover_pending) =
            self.step_node_hover_amounts(dt, is_stage && animations_enabled);
        if hover_changed {
            needs_redraw = true;
        }

        let keep_running = anim_pending || hover_pending;
        if !keep_running {
            self.diagram_tick_active = false;
        }

        (needs_redraw, keep_running)
    }

    pub fn step_pinout_tick(
        &mut self,
        is_stage: bool,
        animations_enabled: bool,
        now: std::time::Instant,
    ) -> (bool, bool) {
        let dt = match self.pin_hover_last_step.replace(now) {
            Some(prev) => now.saturating_duration_since(prev).as_secs_f64(),
            None => 0.0,
        };
        let (changed, pending) = self.step_pin_hover_amounts(dt, is_stage && animations_enabled);
        if !pending {
            self.pinout_tick_active = false;
        }
        (changed, pending)
    }
}

/// Defense-in-depth closure helper for read-access to DiagramCanvasState.
pub fn with_canvas_state<R>(
    cell: &Rc<RefCell<DiagramCanvasState>>,
    f: impl FnOnce(&DiagramCanvasState) -> R,
) -> R {
    let canvas = cell.borrow();
    f(&canvas)
}

/// Defense-in-depth closure helper for mutable access to DiagramCanvasState.
pub fn with_canvas_state_mut<R>(
    cell: &Rc<RefCell<DiagramCanvasState>>,
    f: impl FnOnce(&mut DiagramCanvasState) -> R,
) -> R {
    let mut canvas = cell.borrow_mut();
    f(&mut canvas)
}

pub struct AppState {
    // 4 independent domain cells (narrow blast radius, non-colliding)
    pub project: Rc<RefCell<ProjectState>>,
    pub build_trace: Rc<RefCell<BuildTraceState>>,
    pub serial: Rc<RefCell<SerialState>>,
    pub canvas: Rc<RefCell<DiagramCanvasState>>,
    pub easter_egg_unlocked: bool,
}

impl AppState {
    /// Defense-in-depth closure helper for read-access to DiagramCanvasState.
    pub fn with_canvas_state<R>(&self, f: impl FnOnce(&DiagramCanvasState) -> R) -> R {
        with_canvas_state(&self.canvas, f)
    }

    /// Defense-in-depth closure helper for mutable access to DiagramCanvasState.
    pub fn with_canvas_state_mut<R>(&self, f: impl FnOnce(&mut DiagramCanvasState) -> R) -> R {
        with_canvas_state_mut(&self.canvas, f)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: Rc::new(RefCell::new(ProjectState::default())),
            build_trace: Rc::new(RefCell::new(BuildTraceState::default())),
            serial: Rc::new(RefCell::new(SerialState::default())),
            canvas: Rc::new(RefCell::new(DiagramCanvasState::default())),
            easter_egg_unlocked: false,
        }
    }
}

pub struct AppWidgets {
    pub window: adw::ApplicationWindow,
    pub stack: gtk4::Stack,
    pub toast_overlay: adw::ToastOverlay,
    pub lbl_discovered_dir: gtk4::Label,
    pub lbl_ioc_path: gtk4::Label,
    pub lbl_main_c_path: gtk4::Label,
    pub btn_load: gtk4::Button,
    pub btn_call_graph: gtk4::Button,
    pub btn_nucleo_pinout: gtk4::Button,
    pub lbl_project_name: gtk4::Label,
    pub lbl_mcu_family: gtk4::Label,
    pub lbl_mcu_name: gtk4::Label,
    pub lbl_build_traceability: gtk4::Label,
    pub area_traceability_verify: gtk4::DrawingArea,
    pub lbl_periph_header: gtk4::Label,
    pub lbl_region_header: gtk4::Label,
    pub list_peripherals: gtk4::ListBox,
    pub list_user_regions: gtk4::ListBox,

    // Build & flash widgets
    pub btn_build: gtk4::Button,
    pub btn_build_flash: gtk4::Button,
    pub btn_enable_traceability: gtk4::Button,
    pub build_log_view: gtk4::TextView,
    pub lbl_build_status: gtk4::Label,
    pub area_flash_verify: gtk4::DrawingArea,
    pub btn_clear_log: gtk4::Button,

    // State diagram widgets
    pub diagram_drawing_area: gtk4::DrawingArea,
    #[allow(dead_code)]
    pub btn_fit_to_view: gtk4::Button,
    #[allow(dead_code)]
    pub diagram_scrolled: gtk4::ScrolledWindow,
    pub combo_state_machine: gtk4::DropDown,
    pub lbl_selected_info: gtk4::Label,

    // Nucleo Pinout widgets
    pub pinout_drawing_area: gtk4::DrawingArea,
    pub _pinout_scrolled: gtk4::ScrolledWindow,
    pub combo_pinout_module: gtk4::DropDown,

    // Serial monitor widgets
    #[allow(dead_code)]
    pub btn_serial_monitor: gtk4::Button,
    pub combo_port: gtk4::DropDown,
    pub btn_refresh_ports: gtk4::Button,
    pub combo_baud: gtk4::DropDown,
    pub btn_connect_serial: gtk4::Button,
    pub lbl_serial_status: gtk4::Label,
    pub btn_clear_serial: gtk4::Button,
    pub serial_log_view: gtk4::TextView,
    pub serial_scrolled: gtk4::ScrolledWindow,
    pub entry_command: gtk4::Entry,
    pub btn_send_command: gtk4::Button,
    #[allow(dead_code)]
    pub box_quick_commands: gtk4::Box,
}

pub fn create_icon_button(label_text: &str, icon_name: &str, is_suggested: bool) -> gtk4::Button {
    let bx = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let icon = gtk4::Image::from_icon_name(icon_name);
    bx.append(&icon);

    let btn = gtk4::Button::builder()
        .child(&bx)
        .tooltip_text(label_text)
        .css_classes(vec!["stakhal-btn".to_string()])
        .build();

    btn.set_cursor_from_name(Some("pointer"));

    if is_suggested {
        btn.add_css_class("suggested-action");
    } else {
        btn.add_css_class("flat");
    }

    btn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_state_struct_defaults() {
        let state = AppState::default();
        assert!(state.project.borrow().project_dir.is_none());
        assert!(!state.build_trace.borrow().build_in_progress);
        assert_eq!(state.serial.borrow().selected_serial_baud, 115200);
        assert_eq!(state.with_canvas_state(|c| c.selected_state_machine), 0);
        assert!(!state.easter_egg_unlocked);
    }

    #[test]
    fn test_with_canvas_state_closure_scoping() {
        let state = AppState::default();

        // Mutate via closure helper
        state.with_canvas_state_mut(|c| {
            c.diagram_zoom = 1.5;
            c.diagram_pan_x = 100.0;
        });

        // Verify borrow is dropped immediately after closure exit
        let zoom = state.with_canvas_state(|c| c.diagram_zoom);
        assert_eq!(zoom, 1.5);

        // A second mutable borrow succeeds without panic because previous borrow dropped
        state.with_canvas_state_mut(|c| {
            c.diagram_pan_y = 200.0;
        });
        assert_eq!(state.with_canvas_state(|c| c.diagram_pan_y), 200.0);
    }

    #[test]
    fn test_with_canvas_state_standalone_helper() {
        let canvas_cell = Rc::new(RefCell::new(DiagramCanvasState::default()));
        with_canvas_state_mut(&canvas_cell, |c| {
            c.diagram_zoom = 2.0;
        });
        let zoom = with_canvas_state(&canvas_cell, |c| c.diagram_zoom);
        assert_eq!(zoom, 2.0);
    }

    #[test]
    fn test_diagram_and_pinout_hover_easing_lifecycle() {
        let t0 = std::time::Instant::now();
        let mut c = DiagramCanvasState::default();

        // Idle: nothing unsettled, tick stops immediately
        assert!(!c.has_unsettled_node_hover());
        assert!(!c.has_unsettled_pin_hover());
        assert_eq!(c.step_diagram_tick(true, true, t0), (false, false));
        assert_eq!(c.step_pinout_tick(true, true, t0), (false, false));

        // Start hovering node "GOING" and pin ("CN7", 1) in Stage
        assert!(c.set_diagram_hovered_node(Some("GOING".to_string()), true, t0));
        assert!(c.set_pinout_hovered_pin(Some(("CN7".to_string(), 1)), true, t0));
        assert!(c.has_unsettled_node_hover());
        assert!(c.has_unsettled_pin_hover());
        assert_eq!(c.node_hover_amounts.get("GOING"), Some(&0.0));
        assert_eq!(c.pin_hover_amounts.get(&("CN7".to_string(), 1)), Some(&0.0));
        c.diagram_tick_active = true;
        c.pinout_tick_active = true;

        // Intermediate steps advance smoothly between 0 and 1
        let t1 = t0 + std::time::Duration::from_millis(80);
        let (d_redraw, d_cont) = c.step_diagram_tick(true, true, t1);
        let (p_redraw, p_cont) = c.step_pinout_tick(true, true, t1);
        assert!(d_redraw && d_cont);
        assert!(p_redraw && p_cont);
        let v_node = *c.node_hover_amounts.get("GOING").unwrap();
        let v_pin = *c.pin_hover_amounts.get(&("CN7".to_string(), 1)).unwrap();
        assert!(
            v_node > 0.5 && v_node < 0.75,
            "expected ~0.632 at tau, got {v_node}"
        );
        assert!(
            v_pin > 0.5 && v_pin < 0.75,
            "expected ~0.632 at tau, got {v_pin}"
        );

        // Settle after enough elapsed time (500ms >> tau * ln(100) ~ 368ms)
        let t2 = t0 + std::time::Duration::from_millis(500);
        let (d_redraw2, d_cont2) = c.step_diagram_tick(true, true, t2);
        let (p_redraw2, p_cont2) = c.step_pinout_tick(true, true, t2);
        assert!(
            d_redraw2 && !d_cont2,
            "Settling frame draws 1.0 then breaks"
        );
        assert!(
            p_redraw2 && !p_cont2,
            "Settling frame draws 1.0 then breaks"
        );
        assert!(!c.diagram_tick_active);
        assert!(!c.pinout_tick_active);
        assert_eq!(c.node_hover_amounts.get("GOING"), Some(&1.0));
        assert_eq!(c.pin_hover_amounts.get(&("CN7".to_string(), 1)), Some(&1.0));

        // While holding steady at settled hover: 0 redraws, 0 continuation
        let t3 = t2 + std::time::Duration::from_millis(50);
        assert_eq!(c.step_diagram_tick(true, true, t3), (false, false));
        assert_eq!(c.step_pinout_tick(true, true, t3), (false, false));

        // Leave: transitions back down to 0, then prunes entry and stops tick
        assert!(c.set_diagram_hovered_node(None, true, t3));
        assert!(c.set_pinout_hovered_pin(None, true, t3));
        c.diagram_tick_active = true;
        c.pinout_tick_active = true;

        let t4 = t3 + std::time::Duration::from_millis(80);
        let (d_out1, d_out_cont1) = c.step_diagram_tick(true, true, t4);
        let (p_out1, p_out_cont1) = c.step_pinout_tick(true, true, t4);
        assert!(d_out1 && d_out_cont1);
        assert!(p_out1 && p_out_cont1);
        let fading_val = *c.node_hover_amounts.get("GOING").unwrap();
        assert!(fading_val > 0.25 && fading_val < 0.45);

        // Re-hover mid-fade preserves partial value instead of resetting to 0
        assert!(c.set_diagram_hovered_node(Some("GOING".to_string()), true, t4));
        assert!((*c.node_hover_amounts.get("GOING").unwrap() - fading_val).abs() < 1e-12);

        // Unhover again and let both completely settle off
        assert!(c.set_diagram_hovered_node(None, true, t4));
        let t5 = t4 + std::time::Duration::from_millis(500);
        let (d_done, d_cont_done) = c.step_diagram_tick(true, true, t5);
        let (p_done, p_cont_done) = c.step_pinout_tick(true, true, t5);
        assert!(d_done && !d_cont_done);
        assert!(p_done && !p_cont_done);
        assert!(c.node_hover_amounts.is_empty());
        assert!(c.pin_hover_amounts.is_empty());
        assert!(!c.diagram_tick_active && !c.pinout_tick_active);

        // Reduced motion or Default skin collapses existing map immediately and never continues
        assert!(c.set_diagram_hovered_node(Some("GOING".to_string()), true, t5));
        assert!(c.set_pinout_hovered_pin(Some(("CN7".to_string(), 1)), true, t5));
        assert_eq!(c.step_diagram_tick(true, false, t5), (true, false));
        assert_eq!(c.step_pinout_tick(false, true, t5), (true, false));
        assert!(c.node_hover_amounts.is_empty());
        assert!(c.pin_hover_amounts.is_empty());
    }
}
