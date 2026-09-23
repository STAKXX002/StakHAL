use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use gtk4::prelude::*;
use libadwaita as adw;
use stakhal_core::ir::schema::Project;

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
    pub hovered_state_node: Option<String>,
    pub state_node_positions: std::collections::HashMap<String, (f64, f64)>,
    pub diagram_bounds: (i32, i32),
    pub drag_start_click_pos: (f64, f64),
    pub drag_start_pan_pos: (f64, f64),
    pub hovered_pinout_pin: Option<(String, u8)>,
    pub hovered_pinout_mouse: Option<(f64, f64)>,
    pub selected_pinout_module: Option<String>,
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
            hovered_state_node: None,
            state_node_positions: std::collections::HashMap::new(),
            diagram_bounds: (1200, 800),
            drag_start_click_pos: (0.0, 0.0),
            drag_start_pan_pos: (40.0, 40.0),
            hovered_pinout_pin: None,
            hovered_pinout_mouse: None,
            selected_pinout_module: None,
        }
    }
}

/// Defense-in-depth closure helper for read-access to DiagramCanvasState.
pub fn with_canvas_state<R>(cell: &Rc<RefCell<DiagramCanvasState>>, f: impl FnOnce(&DiagramCanvasState) -> R) -> R {
    let canvas = cell.borrow();
    f(&canvas)
}

/// Defense-in-depth closure helper for mutable access to DiagramCanvasState.
pub fn with_canvas_state_mut<R>(cell: &Rc<RefCell<DiagramCanvasState>>, f: impl FnOnce(&mut DiagramCanvasState) -> R) -> R {
    let mut canvas = cell.borrow_mut();
    f(&mut canvas)
}

pub struct AppState {
    // 4 independent domain cells (narrow blast radius, non-colliding)
    pub project: Rc<RefCell<ProjectState>>,
    pub build_trace: Rc<RefCell<BuildTraceState>>,
    pub serial: Rc<RefCell<SerialState>>,
    pub canvas: Rc<RefCell<DiagramCanvasState>>,

    // State machine diagram state
    pub selected_state_machine: usize,
    pub state_diagram_layout: Option<stakhal_core::graph::StateMachineLayout>,
    pub diagram_zoom: f64,
    pub diagram_pan_x: f64,
    pub diagram_pan_y: f64,
    pub diagram_mouse_pos: Option<(f64, f64)>,
    pub diagram_needs_fit: bool,
    pub selected_state_node: Option<String>,
    pub hovered_state_node: Option<String>,
    pub state_node_positions: std::collections::HashMap<String, (f64, f64)>,
    pub diagram_bounds: (i32, i32),

    pub drag_start_click_pos: (f64, f64),
    pub drag_start_pan_pos: (f64, f64),

    // Nucleo pinout state
    pub hovered_pinout_pin: Option<(String, u8)>,
    pub hovered_pinout_mouse: Option<(f64, f64)>,
    pub selected_pinout_module: Option<String>,

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

            selected_state_machine: 0,
            state_diagram_layout: None,
            diagram_zoom: 0.95,
            diagram_pan_x: 40.0,
            diagram_pan_y: 40.0,
            diagram_mouse_pos: None,
            diagram_needs_fit: true,
            selected_state_node: None,
            hovered_state_node: None,
            state_node_positions: std::collections::HashMap::new(),
            diagram_bounds: (1200, 800),
            drag_start_click_pos: (0.0, 0.0),
            drag_start_pan_pos: (40.0, 40.0),
            hovered_pinout_pin: None,
            hovered_pinout_mouse: None,
            selected_pinout_module: None,
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
}
