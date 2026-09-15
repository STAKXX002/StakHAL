use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::graph::builder::{EdgeType, GraphEdge};
use crate::ioc::parser::PeripheralConfig;
use crate::ir::schema::Project;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateKind {
    Reset,     // Hardware reset / uninitialized state
    Ready,     // Initialized and configured, idle
    Active,    // Listening / enabled for events
    Handling,  // Executing ISR / HAL dispatch
    Callback,  // User / HAL weak callback executing
    Error,     // Error condition encountered
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateNode {
    pub id: String,
    pub name: String,
    pub peripheral: String,
    pub kind: StateKind,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub id: String,
    pub from: String,
    pub to: String,
    pub trigger: String,
    pub action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMachine {
    pub peripheral: String,
    pub states: Vec<StateNode>,
    pub transitions: Vec<StateTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectStateModel {
    pub machines: Vec<StateMachine>,
}

/// Builds a state machine for an individual peripheral based on project call graph edges.
pub fn build_state_machine_for_peripheral(peripheral: &str, edges: &[GraphEdge]) -> StateMachine {
    let mut states = Vec::new();
    let mut transitions = Vec::new();

    let reset_id = format!("{}_RESET", peripheral);
    let ready_id = format!("{}_READY", peripheral);

    states.push(StateNode {
        id: reset_id.clone(),
        name: "RESET".to_string(),
        peripheral: peripheral.to_string(),
        kind: StateKind::Reset,
        description: format!("{} is in hardware reset / uninitialized.", peripheral),
    });

    states.push(StateNode {
        id: ready_id.clone(),
        name: "READY".to_string(),
        peripheral: peripheral.to_string(),
        kind: StateKind::Ready,
        description: format!("{} is initialized and awaiting commands or interrupts.", peripheral),
    });

    let init_target = format!("MX_{}_Init", peripheral);
    let has_init = edges
        .iter()
        .any(|e| e.edge_type == EdgeType::Init && e.to == init_target);

    let init_trigger = if has_init {
        format!("MX_{}_Init()", peripheral)
    } else {
        "Initialize()".to_string()
    };

    transitions.push(StateTransition {
        id: format!("{}_t_init", peripheral),
        from: reset_id.clone(),
        to: ready_id.clone(),
        trigger: init_trigger,
        action: Some("Configure clocks, pins, & parameters".to_string()),
    });

    // Check for IRQ entry edges matching this peripheral
    let irq_entries: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| {
            e.edge_type == EdgeType::IrqEntry && handler_belongs_to_peripheral(&e.from, peripheral)
        })
        .collect();

    if !irq_entries.is_empty() {
        let active_id = format!("{}_ACTIVE", peripheral);
        let isr_id = format!("{}_ISR", peripheral);

        states.push(StateNode {
            id: active_id.clone(),
            name: "ACTIVE".to_string(),
            peripheral: peripheral.to_string(),
            kind: StateKind::Active,
            description: format!("{} NVIC interrupt is enabled, awaiting hardware triggers.", peripheral),
        });

        states.push(StateNode {
            id: isr_id.clone(),
            name: "ISR".to_string(),
            peripheral: peripheral.to_string(),
            kind: StateKind::Handling,
            description: format!("{} IRQ fired; executing HAL dispatch routine.", peripheral),
        });

        transitions.push(StateTransition {
            id: format!("{}_t_arm", peripheral),
            from: ready_id.clone(),
            to: active_id.clone(),
            trigger: "HAL_NVIC_EnableIRQ()".to_string(),
            action: Some("Arm interrupt vector in NVIC".to_string()),
        });

        let irq_names: Vec<String> = irq_entries.iter().map(|e| e.from.clone()).collect();
        let irq_trigger = irq_names.join(" / ");

        transitions.push(StateTransition {
            id: format!("{}_t_fire", peripheral),
            from: active_id.clone(),
            to: isr_id.clone(),
            trigger: irq_trigger,
            action: Some("Context switch to interrupt vector".to_string()),
        });

        // Collect weak callbacks dispatched from the HAL handler
        let mut dispatch_targets = HashSet::new();
        for entry in &irq_entries {
            dispatch_targets.insert(entry.to.as_str());
        }

        let callback_edges: Vec<&GraphEdge> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::WeakOverride && dispatch_targets.contains(e.from.as_str()))
            .collect();

        let mut has_error_state = false;
        let mut cb_idx = 0;

        for cb_edge in &callback_edges {
            let cb_name = &cb_edge.to;
            let is_error = cb_name.contains("Error");

            if is_error {
                if !has_error_state {
                    let err_id = format!("{}_ERROR", peripheral);
                    states.push(StateNode {
                        id: err_id.clone(),
                        name: "ERROR".to_string(),
                        peripheral: peripheral.to_string(),
                        kind: StateKind::Error,
                        description: format!("{} error interrupt triggered.", peripheral),
                    });

                    transitions.push(StateTransition {
                        id: format!("{}_t_err", peripheral),
                        from: isr_id.clone(),
                        to: err_id.clone(),
                        trigger: cb_name.clone(),
                        action: Some("Execute error callback handler".to_string()),
                    });

                    transitions.push(StateTransition {
                        id: format!("{}_t_err_recover", peripheral),
                        from: err_id.clone(),
                        to: ready_id.clone(),
                        trigger: "Clear error / Reset state".to_string(),
                        action: Some("Re-initialize peripheral state".to_string()),
                    });

                    has_error_state = true;
                }
            } else {
                cb_idx += 1;
                let cb_short = shorten_callback_name(cb_name, peripheral);
                let cb_node_id = format!("{}_CB_{}", peripheral, cb_idx);

                states.push(StateNode {
                    id: cb_node_id.clone(),
                    name: cb_short,
                    peripheral: peripheral.to_string(),
                    kind: StateKind::Callback,
                    description: format!("Dispatched callback: {}", cb_name),
                });

                transitions.push(StateTransition {
                    id: format!("{}_t_cb_{}", peripheral, cb_idx),
                    from: isr_id.clone(),
                    to: cb_node_id.clone(),
                    trigger: cb_name.clone(),
                    action: Some("Invoke weak callback in user code".to_string()),
                });

                transitions.push(StateTransition {
                    id: format!("{}_t_cb_ret_{}", peripheral, cb_idx),
                    from: cb_node_id.clone(),
                    to: ready_id.clone(),
                    trigger: "Return from callback".to_string(),
                    action: Some("End of interrupt service".to_string()),
                });
            }
        }

        if callback_edges.is_empty() {
            transitions.push(StateTransition {
                id: format!("{}_t_isr_ret", peripheral),
                from: isr_id.clone(),
                to: ready_id.clone(),
                trigger: "ISR Exit".to_string(),
                action: Some("Acknowledge interrupt & return".to_string()),
            });
        }
    } else {
        // Polling / Synchronous Mode
        let busy_id = format!("{}_BUSY", peripheral);
        states.push(StateNode {
            id: busy_id.clone(),
            name: "BUSY".to_string(),
            peripheral: peripheral.to_string(),
            kind: StateKind::Active,
            description: format!("{} is performing a blocking polling transfer.", peripheral),
        });

        transitions.push(StateTransition {
            id: format!("{}_t_poll_start", peripheral),
            from: ready_id.clone(),
            to: busy_id.clone(),
            trigger: "HAL_Transfer()".to_string(),
            action: Some("Poll status registers until completion".to_string()),
        });

        transitions.push(StateTransition {
            id: format!("{}_t_poll_done", peripheral),
            from: busy_id.clone(),
            to: ready_id.clone(),
            trigger: "Transfer Complete / Timeout".to_string(),
            action: Some("Return transfer status".to_string()),
        });
    }

    StateMachine {
        peripheral: peripheral.to_string(),
        states,
        transitions,
    }
}

/// Builds state machines for all peripherals present in the project.
pub fn build_project_state_model(project: &Project) -> ProjectStateModel {
    build_state_model_from_parts(&project.peripherals, &project.call_graph_edges)
}

pub fn build_state_model_from_parts(
    peripherals: &[PeripheralConfig],
    edges: &[GraphEdge],
) -> ProjectStateModel {
    let mut machines = Vec::new();

    for p in peripherals {
        let sm = build_state_machine_for_peripheral(&p.name, edges);
        machines.push(sm);
    }

    ProjectStateModel { machines }
}

/// Computes (x, y) coordinates for state nodes in a state machine.
pub fn compute_state_machine_layout(
    machine: &StateMachine,
    origin_x: f64,
    origin_y: f64,
) -> HashMap<String, (f64, f64)> {
    let mut positions = HashMap::new();

    let mut callback_count = 0;
    for s in &machine.states {
        if s.kind == StateKind::Callback {
            callback_count += 1;
        }
    }

    let mut cb_seen = 0;

    for s in &machine.states {
        let (x, y) = match s.kind {
            StateKind::Reset => (origin_x + 60.0, origin_y + 120.0),
            StateKind::Ready => (origin_x + 280.0, origin_y + 120.0),
            StateKind::Active => {
                if s.name == "BUSY" {
                    (origin_x + 520.0, origin_y + 120.0)
                } else {
                    (origin_x + 500.0, origin_y + 120.0)
                }
            }
            StateKind::Handling => (origin_x + 720.0, origin_y + 120.0),
            StateKind::Callback => {
                let start_y = if callback_count <= 1 {
                    origin_y + 120.0
                } else {
                    origin_y + 40.0
                };
                let step_y = 90.0;
                let cb_y = start_y + (cb_seen as f64) * step_y;
                cb_seen += 1;
                (origin_x + 960.0, cb_y)
            }
            StateKind::Error => (origin_x + 720.0, origin_y + 250.0),
        };

        positions.insert(s.id.clone(), (x, y));
    }

    positions
}

/// Computes layout positions for the entire project state model.
/// If `selected_peripheral` is provided, only that peripheral's state machine is positioned.
/// Returns `(positions_map, (bounds_width, bounds_height))`.
pub fn compute_project_state_layout(
    model: &ProjectStateModel,
    selected_peripheral: Option<&str>,
) -> (HashMap<String, (f64, f64)>, (i32, i32)) {
    let mut all_positions = HashMap::new();

    let machines_to_layout: Vec<&StateMachine> = match selected_peripheral {
        Some(p) if !p.is_empty() && p != "ALL" => {
            model.machines.iter().filter(|m| m.peripheral == p).collect()
        }
        _ => model.machines.iter().collect(),
    };

    if machines_to_layout.is_empty() {
        return (all_positions, (800, 600));
    }

    let mut current_y = 60.0;
    let block_spacing = 380.0;

    let mut max_x: f64 = 1100.0;
    let mut max_y: f64 = 600.0;

    for sm in &machines_to_layout {
        let pos = compute_state_machine_layout(sm, 40.0, current_y);
        for (id, (x, y)) in pos {
            if x + 160.0 > max_x {
                max_x = x + 160.0;
            }
            if y + 100.0 > max_y {
                max_y = y + 100.0;
            }
            all_positions.insert(id, (x, y));
        }
        current_y += block_spacing;
    }

    let width = (max_x + 120.0).ceil() as i32;
    let height = (max_y + 120.0).ceil() as i32;

    (all_positions, (width.max(800), height.max(600)))
}

fn handler_belongs_to_peripheral(handler_name: &str, periph: &str) -> bool {
    if let Some(rest) = handler_name.strip_prefix(periph) {
        match rest.chars().next() {
            Some(ch) => !ch.is_ascii_digit(),
            None => true,
        }
    } else {
        false
    }
}

fn shorten_callback_name(cb_name: &str, periph: &str) -> String {
    // e.g. "HAL_UART_RxCpltCallback" -> "RxCplt"
    let trimmed = cb_name
        .strip_prefix("HAL_")
        .unwrap_or(cb_name);

    // Remove peripheral prefix if present, e.g. "UART_" or "TIM_"
    let after_prefix = if let Some(pos) = trimmed.find('_') {
        &trimmed[pos + 1..]
    } else {
        trimmed
    };

    let short = after_prefix
        .strip_suffix("Callback")
        .unwrap_or(after_prefix);

    if short.is_empty() {
        periph.to_string()
    } else {
        short.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polling_peripheral_state_machine() {
        let edges = vec![GraphEdge {
            from: "main".to_string(),
            to: "MX_I2C1_Init".to_string(),
            edge_type: EdgeType::Init,
            generated: true,
        }];

        let sm = build_state_machine_for_peripheral("I2C1", &edges);
        assert_eq!(sm.peripheral, "I2C1");
        assert_eq!(sm.states.len(), 3); // RESET, READY, BUSY
        assert_eq!(sm.transitions.len(), 3); // Init, poll_start, poll_done
        assert_eq!(sm.states[0].kind, StateKind::Reset);
        assert_eq!(sm.states[1].kind, StateKind::Ready);
        assert_eq!(sm.states[2].kind, StateKind::Active);
    }

    #[test]
    fn test_irq_peripheral_state_machine() {
        let edges = vec![
            GraphEdge {
                from: "main".to_string(),
                to: "MX_USART2_Init".to_string(),
                edge_type: EdgeType::Init,
                generated: true,
            },
            GraphEdge {
                from: "USART2_IRQHandler".to_string(),
                to: "HAL_UART_IRQHandler".to_string(),
                edge_type: EdgeType::IrqEntry,
                generated: true,
            },
            GraphEdge {
                from: "HAL_UART_IRQHandler".to_string(),
                to: "HAL_UART_RxCpltCallback".to_string(),
                edge_type: EdgeType::WeakOverride,
                generated: true,
            },
            GraphEdge {
                from: "HAL_UART_IRQHandler".to_string(),
                to: "HAL_UART_ErrorCallback".to_string(),
                edge_type: EdgeType::WeakOverride,
                generated: true,
            },
        ];

        let sm = build_state_machine_for_peripheral("USART2", &edges);
        assert_eq!(sm.peripheral, "USART2");

        let state_names: Vec<&str> = sm.states.iter().map(|s| s.name.as_str()).collect();
        assert!(state_names.contains(&"RESET"));
        assert!(state_names.contains(&"READY"));
        assert!(state_names.contains(&"ACTIVE"));
        assert!(state_names.contains(&"ISR"));
        assert!(state_names.contains(&"RxCplt"));
        assert!(state_names.contains(&"ERROR"));

        let positions = compute_state_machine_layout(&sm, 0.0, 0.0);
        assert_eq!(positions.len(), sm.states.len());
    }

    #[test]
    fn test_project_state_layout_computation() {
        let peripherals = vec![
            PeripheralConfig {
                name: "USART2".to_string(),
                mode: Some("Asynchronous".to_string()),
                parameters: HashMap::new(),
            },
            PeripheralConfig {
                name: "TIM2".to_string(),
                mode: None,
                parameters: HashMap::new(),
            },
        ];

        let edges = vec![
            GraphEdge {
                from: "main".to_string(),
                to: "MX_USART2_Init".to_string(),
                edge_type: EdgeType::Init,
                generated: true,
            },
            GraphEdge {
                from: "main".to_string(),
                to: "MX_TIM2_Init".to_string(),
                edge_type: EdgeType::Init,
                generated: true,
            },
        ];

        let model = build_state_model_from_parts(&peripherals, &edges);
        assert_eq!(model.machines.len(), 2);

        let (all_pos, bounds) = compute_project_state_layout(&model, None);
        assert!(bounds.0 >= 800);
        assert!(bounds.1 >= 600);
        assert!(!all_pos.is_empty());

        let (single_pos, _) = compute_project_state_layout(&model, Some("USART2"));
        assert_eq!(single_pos.len(), 3); // USART2 polling states
    }
}
