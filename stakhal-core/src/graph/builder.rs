use serde::{Deserialize, Serialize};
use crate::graph::hal_rules::HAL_IRQ_MAPPINGS;
use crate::ioc::IocProject;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Init,
    IrqEntry,
    HalDispatch,
    WeakOverride,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub generated: bool,
}

/// Helper function to check whether an IRQ handler name belongs to a peripheral instance.
///
/// Strips `instance_name` from `handler_name`. If `handler_name` starts with `instance_name`,
/// the next character immediately following `instance_name` must NOT be an ASCII digit.
/// For example, "TIM1_UP_TIM10_IRQHandler" matches "TIM1" (next char is '_'),
/// but "TIM10_IRQHandler" does not match "TIM1" (next char is '0').
fn handler_matches_instance(handler_name: &str, instance_name: &str) -> bool {
    if let Some(rest) = handler_name.strip_prefix(instance_name) {
        match rest.chars().next() {
            Some(ch) => !ch.is_ascii_digit(),
            None => true,
        }
    } else {
        false
    }
}

pub fn build_call_graph(ioc: &IocProject) -> Vec<GraphEdge> {
    let mut edges = Vec::new();

    for peripheral in &ioc.peripherals {
        // 1. Always emit Init edge for every peripheral
        edges.push(GraphEdge {
            from: "main".to_string(),
            to: format!("MX_{}_Init", peripheral.name),
            edge_type: EdgeType::Init,
            generated: true,
        });

        // 2. Find candidate HalIrqMapping entries for this peripheral instance
        for mapping in HAL_IRQ_MAPPINGS {
            if handler_matches_instance(mapping.irq_handler_name, &peripheral.name) {
                // 3. Derive NVIC key e.g. "USART2_IRQHandler" -> "NVIC.USART2_IRQn"
                let stem = mapping
                    .irq_handler_name
                    .strip_suffix("_IRQHandler")
                    .unwrap_or(mapping.irq_handler_name);
                let nvic_key = format!("NVIC.{stem}_IRQn");

                // Check if NVIC key exists in ioc.raw AND its value starts with "true"
                let is_enabled = ioc
                    .raw
                    .get(&nvic_key)
                    .map(|v| v.starts_with("true"))
                    .unwrap_or(false);

                if is_enabled {
                    // 4. Emit IrqEntry and WeakOverride edges
                    edges.push(GraphEdge {
                        from: mapping.irq_handler_name.to_string(),
                        to: mapping.hal_dispatch_fn.to_string(),
                        edge_type: EdgeType::IrqEntry,
                        generated: true,
                    });

                    for &cb in mapping.weak_callbacks {
                        edges.push(GraphEdge {
                            from: mapping.hal_dispatch_fn.to_string(),
                            to: cb.to_string(),
                            edge_type: EdgeType::WeakOverride,
                            generated: true,
                        });
                    }
                }
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::ioc::PeripheralConfig;

    fn make_test_ioc(nvic_value: Option<&str>) -> IocProject {
        let mut raw = HashMap::new();
        if let Some(val) = nvic_value {
            raw.insert("NVIC.USART2_IRQn".to_string(), val.to_string());
        }

        IocProject {
            mcu_family: "STM32F4".to_string(),
            mcu_name: "STM32F446RETx".to_string(),
            pins: vec![],
            peripherals: vec![PeripheralConfig {
                name: "USART2".to_string(),
                mode: Some("Asynchronous".to_string()),
                parameters: HashMap::new(),
            }],
            raw,
        }
    }

    #[test]
    fn test_nvic_enabled_emits_full_chain() {
        let ioc = make_test_ioc(Some("true:5:0:true:false:true:true"));
        let edges = build_call_graph(&ioc);

        assert!(edges.iter().any(|e| e.from == "main"
            && e.to == "MX_USART2_Init"
            && e.edge_type == EdgeType::Init));
        assert!(edges.iter().any(|e| e.from == "USART2_IRQHandler"
            && e.to == "HAL_UART_IRQHandler"
            && e.edge_type == EdgeType::IrqEntry));
        assert!(edges.iter().any(|e| e.from == "HAL_UART_IRQHandler"
            && e.to == "HAL_UART_RxCpltCallback"
            && e.edge_type == EdgeType::WeakOverride));
    }

    #[test]
    fn test_nvic_disabled_emits_only_init() {
        let ioc = make_test_ioc(Some("false:5:0:true:false:true:true"));
        let edges = build_call_graph(&ioc);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "main");
        assert_eq!(edges[0].to, "MX_USART2_Init");
        assert_eq!(edges[0].edge_type, EdgeType::Init);
    }

    #[test]
    fn test_nvic_absent_emits_only_init() {
        let ioc = make_test_ioc(None);
        let edges = build_call_graph(&ioc);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "main");
        assert_eq!(edges[0].to, "MX_USART2_Init");
        assert_eq!(edges[0].edge_type, EdgeType::Init);
    }
}
