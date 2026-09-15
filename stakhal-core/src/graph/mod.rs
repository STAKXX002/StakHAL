pub mod builder;
pub mod hal_rules;
pub mod layout;
pub mod state_transition;
pub mod user_call_graph;

pub use builder::{build_call_graph, EdgeType, GraphEdge};
pub use hal_rules::{
    mapping_for_irq_handler, mappings_for_peripheral_prefix, HalIrqMapping, HAL_IRQ_MAPPINGS,
};
pub use layout::{compute_graph_bounds, compute_graph_layout, ChainHeaderLayout};
pub use state_transition::{
    build_project_state_model, build_state_machine_for_peripheral, build_state_model_from_parts,
    compute_project_state_layout, compute_state_machine_layout, ProjectStateModel, StateKind,
    StateMachine, StateNode, StateTransition,
};
pub use user_call_graph::{build_user_call_graph, UserCallEdge, UserFunction};

