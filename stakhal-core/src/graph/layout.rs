use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use super::builder::{EdgeType, GraphEdge};
use super::hal_rules::mapping_for_irq_handler;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChainHeaderLayout {
    pub handler_id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub is_collapsed: bool,
}

#[derive(Debug)]
struct IrqChainInfo {
    handler_id: String,
    dispatch_id: String,
    override_targets: Vec<String>,
}

#[derive(Debug)]
struct PeripheralLane {
    init_node: Option<String>,
    irq_chains: Vec<IrqChainInfo>,
}


fn infer_peripheral_name(handler_id: &str, known_init_periphs: &[String]) -> String {
    for p in known_init_periphs {
        if let Some(rest) = handler_id.strip_prefix(p) {
            if rest.is_empty() || rest.starts_with('_') || !rest.chars().next().unwrap().is_ascii_digit() {
                return p.clone();
            }
        }
    }

    if handler_id.starts_with("EXTI") {
        return "GPIO".to_string();
    }

    if let Some(mapping) = mapping_for_irq_handler(handler_id) {
        if mapping.peripheral_prefix == "GPIO" {
            return "GPIO".to_string();
        }
    }

    if let Some(stem) = handler_id.strip_suffix("_IRQHandler") {
        if let Some(first_part) = stem.split('_').next() {
            return first_part.to_string();
        }
        return stem.to_string();
    }

    "PERIPHERAL".to_string()
}

pub fn compute_graph_layout(
    edges: &[GraphEdge],
    collapsed_chains: &HashSet<String>,
) -> (HashMap<String, (f64, f64)>, Vec<ChainHeaderLayout>) {
    let mut map = HashMap::new();
    let mut headers = Vec::new();

    if edges.is_empty() {
        return (map, headers);
    }

    // 1. Collect Init edges: main -> MX_<PERIPH>_Init
    let init_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Init)
        .collect();

    let mut init_periph_names = Vec::new();
    let mut periph_init_map: HashMap<String, String> = HashMap::new();

    for e in &init_edges {
        let node_name = &e.to;
        let periph_name = node_name
            .strip_prefix("MX_")
            .and_then(|s| s.strip_suffix("_Init"))
            .unwrap_or(node_name)
            .to_string();

        if !init_periph_names.contains(&periph_name) {
            init_periph_names.push(periph_name.clone());
        }
        periph_init_map.insert(periph_name, node_name.clone());
    }

    // 2. Collect IRQ entry chains: handler_id -> dispatch_id -> override_targets
    let irq_entry_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::IrqEntry)
        .collect();

    let mut irq_chains_by_periph: HashMap<String, Vec<IrqChainInfo>> = HashMap::new();

    for irq_edge in &irq_entry_edges {
        let handler_id = irq_edge.from.clone();
        let dispatch_id = irq_edge.to.clone();

        let override_targets: Vec<String> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::WeakOverride && e.from == dispatch_id)
            .map(|e| e.to.clone())
            .collect();

        let periph_name = infer_peripheral_name(&handler_id, &init_periph_names);

        irq_chains_by_periph
            .entry(periph_name)
            .or_default()
            .push(IrqChainInfo {
                handler_id,
                dispatch_id,
                override_targets,
            });
    }

    // 3. Assemble ordered peripheral lanes
    let mut lane_names: Vec<String> = init_periph_names.clone();
    for p in irq_chains_by_periph.keys() {
        if !lane_names.contains(p) {
            lane_names.push(p.clone());
        }
    }
    // Sort lanes: GPIO first, then alphabetically
    lane_names.sort_by(|a, b| {
        if a == "GPIO" {
            std::cmp::Ordering::Less
        } else if b == "GPIO" {
            std::cmp::Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    let mut lanes: Vec<PeripheralLane> = Vec::new();
    for name in lane_names {
        let init_node = periph_init_map.get(&name).cloned();
        let irq_chains = irq_chains_by_periph.remove(&name).unwrap_or_default();
        lanes.push(PeripheralLane {
            init_node,
            irq_chains,
        });
    }

    // 4. Lay out swimlanes left-to-right and top-to-bottom
    let lane_spacing_x = 40.0;
    let mut current_lane_x = 40.0;
    let mut lane_x_bounds: Vec<(f64, f64)> = Vec::new();

    for lane in &lanes {
        let mut lane_nodes_w: Vec<f64> = Vec::new();
        if let Some(ref init_id) = lane.init_node {
            lane_nodes_w.push((init_id.len() as f64 * 8.5 + 28.0).max(110.0));
        }

        for chain in &lane.irq_chains {
            let total_nodes = 1 + 1 + chain.override_targets.len();
            let header_label = format!("{} chain ({} nodes)", chain.handler_id, total_nodes);
            lane_nodes_w.push((header_label.len() as f64 * 8.0 + 36.0).max(180.0));
            lane_nodes_w.push((chain.handler_id.len() as f64 * 8.5 + 28.0).max(120.0));
            lane_nodes_w.push((chain.dispatch_id.len() as f64 * 8.5 + 28.0).max(120.0));
            for ov in &chain.override_targets {
                lane_nodes_w.push((ov.len() as f64 * 8.5 + 28.0).max(120.0));
            }
        }

        let max_node_w = lane_nodes_w
            .into_iter()
            .fold(160.0f64, |acc, x| acc.max(x));

        let lane_w = max_node_w;
        let lane_start_x = current_lane_x;
        let lane_center_x = lane_start_x + lane_w / 2.0;

        let mut current_y = 140.0;

        // Depth 0: Init node
        if let Some(ref init_id) = lane.init_node {
            let node_w = (init_id.len() as f64 * 8.5 + 28.0).max(110.0);
            map.insert(init_id.clone(), (lane_center_x - node_w / 2.0, current_y));
            current_y += 34.0 + 35.0;
        }

        // Depths 1, 2, 3+: IRQ chains
        for chain in &lane.irq_chains {
            let is_collapsed = collapsed_chains.contains(&chain.handler_id);
            let total_nodes = 1 + 1 + chain.override_targets.len();
            let header_label = format!("{} chain ({} nodes)", chain.handler_id, total_nodes);
            let header_w = (header_label.len() as f64 * 8.0 + 36.0).max(180.0);
            let header_h = 28.0;

            headers.push(ChainHeaderLayout {
                handler_id: chain.handler_id.clone(),
                label: header_label,
                x: lane_center_x - header_w / 2.0,
                y: current_y,
                w: header_w,
                h: header_h,
                is_collapsed,
            });

            current_y += header_h + 16.0;

            if !is_collapsed {
                let handler_w = (chain.handler_id.len() as f64 * 8.5 + 28.0).max(120.0);
                map.insert(
                    chain.handler_id.clone(),
                    (lane_center_x - handler_w / 2.0, current_y),
                );
                current_y += 34.0 + 25.0;

                let dispatch_w = (chain.dispatch_id.len() as f64 * 8.5 + 28.0).max(120.0);
                map.insert(
                    chain.dispatch_id.clone(),
                    (lane_center_x - dispatch_w / 2.0, current_y),
                );
                current_y += 34.0 + 25.0;

                for ov in &chain.override_targets {
                    let ov_w = (ov.len() as f64 * 8.5 + 28.0).max(120.0);
                    map.insert(ov.clone(), (lane_center_x - ov_w / 2.0, current_y));
                    current_y += 34.0 + 15.0;
                }

                current_y += 15.0;
            }
        }

        lane_x_bounds.push((lane_start_x, lane_start_x + lane_w));
        current_lane_x += lane_w + lane_spacing_x;
    }

    // 5. Position main centered above all lanes
    let (first_x, last_x) = if let (Some(f), Some(l)) = (lane_x_bounds.first(), lane_x_bounds.last()) {
        (f.0, l.1)
    } else {
        (40.0, 200.0)
    };

    let main_w = 110.0;
    let main_center_x = (first_x + last_x) / 2.0;
    let main_x = (main_center_x - main_w / 2.0).max(40.0);
    map.insert("main".to_string(), (main_x, 50.0));

    (map, headers)
}

pub fn compute_graph_bounds(
    positions: &HashMap<String, (f64, f64)>,
    headers: &[ChainHeaderLayout],
) -> (i32, i32) {
    let mut max_x = 0.0f64;
    let mut max_y = 0.0f64;

    for (id, &(x, y)) in positions {
        let w = (id.len() as f64 * 8.5 + 28.0).max(110.0);
        let h = 34.0;
        if x + w > max_x {
            max_x = x + w;
        }
        if y + h > max_y {
            max_y = y + h;
        }
    }

    for h in headers {
        if h.x + h.w > max_x {
            max_x = h.x + h.w;
        }
        if h.y + h.h > max_y {
            max_y = h.y + h.h;
        }
    }

    if max_x == 0.0 && max_y == 0.0 {
        (800, 600)
    } else {
        let w = (max_x + 60.0).ceil() as i32;
        let h = (max_y + 60.0).ceil() as i32;
        (w.max(800), h.max(600))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_edge_list_returns_empty_layout() {
        let edges: Vec<GraphEdge> = Vec::new();
        let collapsed = HashSet::new();
        let (pos, headers) = compute_graph_layout(&edges, &collapsed);
        assert!(pos.is_empty(), "Position map should be empty for no edges");
        assert!(headers.is_empty(), "Chain headers should be empty for no edges");
    }

    #[test]
    fn test_simple_init_chain_layout_produces_main_node() {
        let edges = vec![GraphEdge {
            from: "main".to_string(),
            to: "MX_GPIO_Init".to_string(),
            edge_type: EdgeType::Init,
            generated: true,
        }];
        let collapsed = HashSet::new();
        let (pos, _headers) = compute_graph_layout(&edges, &collapsed);

        assert!(pos.contains_key("main"), "Layout must position 'main' node");
        assert!(pos.contains_key("MX_GPIO_Init"), "Layout must position target init node");

        let main_pos = pos.get("main").unwrap();
        assert_eq!(main_pos.1, 50.0, "Main node should be placed at y=50.0");

        let init_pos = pos.get("MX_GPIO_Init").unwrap();
        assert!(init_pos.1 > main_pos.1, "Init node must be positioned below main node (depth 0)");
    }

    #[test]
    fn test_swimlane_grouping_and_relative_depth_ordering() {
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
        ];

        let collapsed = HashSet::new();
        let (pos, headers) = compute_graph_layout(&edges, &collapsed);

        assert_eq!(headers.len(), 1, "Should generate 1 chain header");
        assert_eq!(headers[0].handler_id, "USART2_IRQHandler");

        let main_y = pos.get("main").unwrap().1;
        let init_y = pos.get("MX_USART2_Init").unwrap().1;
        let irq_y = pos.get("USART2_IRQHandler").unwrap().1;
        let dispatch_y = pos.get("HAL_UART_IRQHandler").unwrap().1;
        let callback_y = pos.get("HAL_UART_RxCpltCallback").unwrap().1;

        assert!(main_y < init_y, "main (y={main_y}) must be above Init (y={init_y})");
        assert!(init_y < irq_y, "Init (y={init_y}) must be above IRQ Handler (y={irq_y})");
        assert!(irq_y < dispatch_y, "IRQ Handler (y={irq_y}) must be above HAL Dispatch (y={dispatch_y})");
        assert!(dispatch_y < callback_y, "HAL Dispatch (y={dispatch_y}) must be above Callback (y={callback_y})");

        // Assert all nodes in USART2 lane share the same center X alignment
        let init_x = pos.get("MX_USART2_Init").unwrap().0;
        let irq_x = pos.get("USART2_IRQHandler").unwrap().0;
        let dispatch_x = pos.get("HAL_UART_IRQHandler").unwrap().0;

        let init_center = init_x + ("MX_USART2_Init".len() as f64 * 8.5 + 28.0).max(110.0) / 2.0;
        let irq_center = irq_x + ("USART2_IRQHandler".len() as f64 * 8.5 + 28.0).max(120.0) / 2.0;
        let dispatch_center = dispatch_x + ("HAL_UART_IRQHandler".len() as f64 * 8.5 + 28.0).max(120.0) / 2.0;

        assert!((init_center - irq_center).abs() < 1.0, "Init and IRQ handler should be centered in the same lane");
        assert!((irq_center - dispatch_center).abs() < 1.0, "IRQ handler and HAL dispatch should be centered in the same lane");
    }

    #[test]
    fn test_compute_graph_bounds_adds_padding() {
        let mut positions = HashMap::new();
        positions.insert("main".to_string(), (100.0, 100.0));
        let headers = vec![ChainHeaderLayout {
            handler_id: "TIM1".to_string(),
            label: "TIM1".to_string(),
            x: 40.0,
            y: 300.0,
            w: 200.0,
            h: 30.0,
            is_collapsed: false,
        }];

        let (w, h) = compute_graph_bounds(&positions, &headers);
        assert!(w >= 800, "Width should be at least minimum 800");
        assert!(h >= 390, "Height should include max_y + padding");
    }
}


