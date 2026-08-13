use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use super::builder::{EdgeType, GraphEdge};

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

pub fn compute_graph_layout(
    edges: &[GraphEdge],
    collapsed_chains: &HashSet<String>,
) -> (HashMap<String, (f64, f64)>, Vec<ChainHeaderLayout>) {
    let mut map = HashMap::new();
    let mut headers = Vec::new();

    let max_row_w = 720.0;

    // 1. INITIALIZATION SECTION
    let init_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Init)
        .collect();

    let mut init_bottom_y = 160.0;

    if !init_edges.is_empty() {
        let mut target_nodes: Vec<String> = init_edges.iter().map(|e| e.to.clone()).collect();
        target_nodes.sort();
        target_nodes.dedup();

        let spacing_x = 20.0;
        let row_height = 55.0;
        let mut rows: Vec<Vec<(String, f64)>> = Vec::new();

        let mut current_row: Vec<(String, f64)> = Vec::new();
        let mut current_row_w = 0.0;

        for target in &target_nodes {
            let w = (target.len() as f64 * 8.5 + 28.0).max(110.0);
            if !current_row.is_empty()
                && (current_row_w + spacing_x + w > max_row_w || current_row.len() >= 5)
            {
                rows.push(current_row);
                current_row = Vec::new();
                current_row_w = 0.0;
            }
            current_row_w += if current_row.is_empty() {
                w
            } else {
                spacing_x + w
            };
            current_row.push((target.clone(), w));
        }
        if !current_row.is_empty() {
            rows.push(current_row);
        }

        let first_row_w = if let Some(r0) = rows.first() {
            r0.iter().map(|(_, w)| w).sum::<f64>()
                + (r0.len().saturating_sub(1) as f64 * spacing_x)
        } else {
            110.0
        };

        let main_w = 110.0;
        let main_x = (40.0 + (first_row_w / 2.0) - (main_w / 2.0)).max(40.0);
        map.insert("main".to_string(), (main_x, 50.0));

        let mut row_start_y = 130.0;
        for row in rows {
            let mut curr_x = 40.0;
            for (id, w) in row {
                map.insert(id, (curr_x, row_start_y));
                curr_x += w + spacing_x;
            }
            row_start_y += row_height;
        }
        init_bottom_y = row_start_y;
    }

    // 2. INTERRUPT CHAINS SECTION (Stacked vertically, left-aligned with collapsible header bars)
    let irq_entry_edges: Vec<&GraphEdge> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::IrqEntry)
        .collect();

    let mut current_chain_y = init_bottom_y + 35.0;

    for irq_edge in &irq_entry_edges {
        let handler_id = irq_edge.from.clone();
        let dispatch_id = irq_edge.to.clone();

        let override_targets: Vec<String> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::WeakOverride && e.from == dispatch_id)
            .map(|e| e.to.clone())
            .collect();

        let total_nodes = 1 + 1 + override_targets.len();
        let is_collapsed = collapsed_chains.contains(&handler_id);

        let header_label = format!("{} chain ({} nodes)", handler_id, total_nodes);
        let header_w = (header_label.len() as f64 * 8.0 + 36.0).max(220.0);
        let header_h = 28.0;
        let chain_x = 40.0;

        headers.push(ChainHeaderLayout {
            handler_id: handler_id.clone(),
            label: header_label,
            x: chain_x,
            y: current_chain_y,
            w: header_w,
            h: header_h,
            is_collapsed,
        });

        if is_collapsed {
            current_chain_y += header_h + 16.0;
        } else {
            let handler_w = (handler_id.len() as f64 * 8.5 + 28.0).max(130.0);
            let dispatch_w = (dispatch_id.len() as f64 * 8.5 + 28.0).max(130.0);

            let chain_y_l1 = current_chain_y + 38.0;
            let chain_y_l2 = chain_y_l1 + 65.0;
            let chain_y_l3 = chain_y_l2 + 65.0;

            let mut override_w_sum = 0.0;
            let mut override_nodes_info = Vec::new();
            let mut curr_ov_x = chain_x;

            for ov in &override_targets {
                let w = (ov.len() as f64 * 8.5 + 28.0).max(130.0);
                override_nodes_info.push((ov.clone(), curr_ov_x, w));
                curr_ov_x += w + 20.0;
                override_w_sum += w + 20.0;
            }

            let chain_max_width = handler_w.max(dispatch_w).max(override_w_sum).max(160.0);
            let center_x = chain_x + (chain_max_width / 2.0);

            map.insert(handler_id, (center_x - (handler_w / 2.0), chain_y_l1));
            map.insert(dispatch_id, (center_x - (dispatch_w / 2.0), chain_y_l2));

            for (ov_id, ov_x, _) in override_nodes_info {
                map.insert(ov_id, (ov_x, chain_y_l3));
            }

            let chain_height = if override_targets.is_empty() {
                130.0
            } else {
                195.0
            };

            current_chain_y += 38.0 + chain_height + 25.0;
        }
    }

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

