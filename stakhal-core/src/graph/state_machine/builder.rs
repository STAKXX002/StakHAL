use std::collections::HashMap;

use petgraph::stable_graph::{NodeIndex, StableDiGraph};

use super::model::{AppStateMachine, StateNode, TransitionEdge};
use super::transition::is_fault_state;

/// Infer semantic cluster name for labelling and UI lane banners.
pub fn infer_cluster_name(state: &str, initial_name: &str) -> &'static str {
    if is_fault_state(state) {
        "FAULT"
    } else if state == initial_name {
        "INITIAL"
    } else if state.starts_with("CAL") {
        "CALIBRATION"
    } else if state.starts_with("REC") {
        "RECOVERY"
    } else if state == "GOING" || state == "HOLD" || state == "RETURNING" || state == "RETURNED" {
        "MOTION"
    } else if state == "OPENING" || state == "CLOSING" {
        "MECHANISM"
    } else {
        "GENERAL"
    }
}

/// Build a petgraph StableDiGraph from an AppStateMachine.
/// Excludes high-fan-in FAULT edges (handled as badges) and wildcard/self transitions.
pub fn build_state_machine_graph(
    sm: &AppStateMachine,
) -> (StableDiGraph<StateNode, TransitionEdge>, HashMap<String, NodeIndex>) {
    let initial_name = sm.var.initial_value.as_deref().unwrap_or("IDLE");
    let mut graph = StableDiGraph::new();
    let mut node_map = HashMap::new();

    for state in &sm.states {
        let is_initial = state == initial_name;
        let is_fault = is_fault_state(state);
        let cluster = infer_cluster_name(state, initial_name).to_string();
        let idx = graph.add_node(StateNode {
            id: state.clone(),
            is_initial,
            is_fault,
            cluster,
        });
        node_map.insert(state.clone(), idx);
    }

    for t in &sm.transitions {
        if !t.is_fault && !is_fault_state(&t.to) && t.from != "(any state)" && t.from != t.to {
            if let (Some(&from_idx), Some(&to_idx)) = (node_map.get(&t.from), node_map.get(&t.to)) {
                graph.add_edge(
                    from_idx,
                    to_idx,
                    TransitionEdge {
                        guard: t.guard.clone(),
                        is_fault: t.is_fault,
                        transition_type: t.transition_type.clone(),
                    },
                );
            }
        }
    }

    (graph, node_map)
}
