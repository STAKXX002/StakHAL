pub mod builder;
pub mod hal_rules;
pub mod user_call_graph;

pub use builder::{build_call_graph, EdgeType, GraphEdge};
pub use hal_rules::{
    mapping_for_irq_handler, mappings_for_peripheral_prefix, HalIrqMapping, HAL_IRQ_MAPPINGS,
};
pub use user_call_graph::{build_user_call_graph, UserCallEdge, UserFunction};
