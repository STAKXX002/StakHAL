use std::collections::{HashMap, HashSet};

use petgraph::stable_graph::{NodeIndex, StableDiGraph};
use rust_sugiyama::configure::{Config, CrossingMinimization};

use super::builder::infer_cluster_name;
use super::model::*;
use super::transition::is_fault_state;

/// Compute layered node-link graph layout with hub-anchored swimlanes.
pub fn compute_state_machine_layout(sm: &AppStateMachine) -> StateMachineLayout {
    let node_width = 175.0;
    let node_height = 54.0;
    let initial_name = sm.var.initial_value.as_deref().unwrap_or("IDLE");

    // 1. Calculate operational and total in/out degrees
    let mut total_in_degrees: HashMap<String, usize> = HashMap::new();
    let mut total_out_degrees: HashMap<String, usize> = HashMap::new();
    let mut op_in_degrees: HashMap<String, usize> = HashMap::new();
    let mut op_out_degrees: HashMap<String, usize> = HashMap::new();
    for st in &sm.states {
        total_in_degrees.insert(st.clone(), 0);
        total_out_degrees.insert(st.clone(), 0);
        op_in_degrees.insert(st.clone(), 0);
        op_out_degrees.insert(st.clone(), 0);
    }
    for t in &sm.transitions {
        if t.from != t.to {
            if t.from != "(any state)" {
                *total_out_degrees.entry(t.from.clone()).or_default() += 1;
            }
            *total_in_degrees.entry(t.to.clone()).or_default() += 1;

            if !t.is_fault && !is_fault_state(&t.to) {
                if t.from != "(any state)" {
                    *op_out_degrees.entry(t.from.clone()).or_default() += 1;
                }
                *op_in_degrees.entry(t.to.clone()).or_default() += 1;
            }
        }
    }

    // High fan-in threshold for badge-only collapsing (e.g. 12 transitions to FAULT)
    let mut high_fan_in_nodes = HashSet::new();
    for st in &sm.states {
        let total_in = sm.transitions.iter().filter(|t| t.to == *st && t.from != *st).count();
        if total_in >= 4 || is_fault_state(st) {
            high_fan_in_nodes.insert(st.clone());
        }
    }

    let mut collapsed_out_map: HashMap<String, Vec<String>> = HashMap::new();
    for t in &sm.transitions {
        if t.from != "(any state)" && high_fan_in_nodes.contains(&t.to) && t.from != t.to {
            let list = collapsed_out_map.entry(t.from.clone()).or_default();
            if !list.contains(&t.to) {
                list.push(t.to.clone());
            }
        }
    }

    // Identify hub nodes:
    // 1. Initial state (e.g. IDLE) is always the entry hub.
    // 2. An exit hub must be an architectural junction bridging multiple distinct functional clusters
    //    (i.e. connects across >= 2 foreign clusters, excluding FAULT and INITIAL).
    //    States with high internal connectivity within a single lane (e.g. OPENING <-> CLOSING)
    //    belong in their functional swimlane, not as global hubs.
    let mut hub_nodes = HashSet::new();
    hub_nodes.insert(initial_name.to_string());

    for st in &sm.states {
        if is_fault_state(st) || st == initial_name {
            continue;
        }
        let my_cluster = infer_cluster_name(st, initial_name);
        let mut foreign_clusters = HashSet::new();
        for t in &sm.transitions {
            if t.from == *st && t.to != *st && !is_fault_state(&t.to) && t.to != initial_name {
                let other_cluster = infer_cluster_name(&t.to, initial_name);
                if other_cluster != my_cluster {
                    foreign_clusters.insert(other_cluster);
                }
            }
            if t.to == *st && t.from != *st && !is_fault_state(&t.from) && t.from != initial_name && t.from != "(any state)" {
                let other_cluster = infer_cluster_name(&t.from, initial_name);
                if other_cluster != my_cluster {
                    foreign_clusters.insert(other_cluster);
                }
            }
        }
        let op_deg = op_in_degrees.get(st).copied().unwrap_or(0) + op_out_degrees.get(st).copied().unwrap_or(0);
        if foreign_clusters.len() >= 2 && op_deg >= 4 {
            hub_nodes.insert(st.clone());
        }
    }

    // Designate entry hub: initial state if present, else hub with highest operational out-degree
    let entry_hub = if hub_nodes.contains(initial_name) {
        initial_name.to_string()
    } else {
        hub_nodes
            .iter()
            .max_by_key(|h| op_out_degrees.get(*h).copied().unwrap_or(0))
            .cloned()
            .unwrap_or_else(|| initial_name.to_string())
    };

    let mut exit_hubs: Vec<String> = hub_nodes.iter().filter(|h| **h != entry_hub).cloned().collect();
    exit_hubs.sort();

    // 2. Partition non-hub, non-fault states into logical flow lanes
    let mut lane_clusters: Vec<(&'static str, Vec<String>)> = vec![
        ("CALIBRATION", Vec::new()),
        ("MECHANISM", Vec::new()),
        ("MOTION", Vec::new()),
        ("RECOVERY", Vec::new()),
    ];
    let mut other_states = Vec::new();

    for st in &sm.states {
        if is_fault_state(st) || hub_nodes.contains(st) {
            continue;
        }
        let cluster = infer_cluster_name(st, initial_name);
        if let Some((_, list)) = lane_clusters.iter_mut().find(|(c, _)| *c == cluster) {
            list.push(st.clone());
        } else {
            other_states.push(st.clone());
        }
    }
    if !other_states.is_empty() {
        lane_clusters.push(("GENERAL", other_states));
    }
    lane_clusters.retain(|(_, list)| !list.is_empty());

    // 3. Run rust-sugiyama on each lane independently
    let config = Config {
        vertex_spacing: 80.0,
        c_minimization: CrossingMinimization::Median,
        transpose: true,
        ..Default::default()
    };
    let size_fn = |_idx: NodeIndex, _node: &StateNode| (node_height, node_width);

    let mut lane_node_ranks: HashMap<String, usize> = HashMap::new();
    let mut max_rank_in_any_lane = 0;

    for (_cluster_name, lane_states) in &lane_clusters {
        let lane_state_set: HashSet<String> = lane_states.iter().cloned().collect();
        let mut lane_graph = StableDiGraph::new();
        let mut lane_map = HashMap::new();

        for st in lane_states {
            let idx = lane_graph.add_node(StateNode {
                id: st.clone(),
                is_initial: false,
                is_fault: false,
                cluster: infer_cluster_name(st, initial_name).to_string(),
            });
            lane_map.insert(st.clone(), idx);
        }

        // Intra-lane edges only
        for t in &sm.transitions {
            if lane_state_set.contains(&t.from) && lane_state_set.contains(&t.to) && t.from != t.to {
                lane_graph.add_edge(
                    lane_map[&t.from],
                    lane_map[&t.to],
                    TransitionEdge {
                        guard: t.guard.clone(),
                        is_fault: false,
                        transition_type: t.transition_type.clone(),
                    },
                );
            }
        }

        let layouts = rust_sugiyama::from_graph(&lane_graph, &size_fn, &config);
        let mut sorted_nodes: Vec<(String, f64)> = Vec::new();
        for (nodes, _w, _h) in layouts {
            for (node_idx, (_sx, sy)) in nodes {
                sorted_nodes.push((lane_graph[node_idx].id.clone(), sy));
            }
        }
        sorted_nodes.sort_by(|a, b| a.1.total_cmp(&b.1));

        for (rank, (name, _)) in sorted_nodes.into_iter().enumerate() {
            lane_node_ranks.insert(name, rank);
            if rank > max_rank_in_any_lane {
                max_rank_in_any_lane = rank;
            }
        }
    }

    // 4. Geometry & Positioning of Hubs and Swimlanes
    let left_margin = 110.0;
    let top_margin = 110.0;
    let lane_gap = 130.0;
    let hub_to_lane_gap = 140.0;
    let rank_step_x = node_width + 80.0; // 255.0

    let lane_start_x = left_margin + node_width + hub_to_lane_gap; // 110 + 175 + 140 = 425.0

    let mut nodes_layout = HashMap::new();
    let mut lane_y_positions: HashMap<String, f64> = HashMap::new();

    // 4a. Position lane nodes in horizontal rows
    for (lane_idx, (cluster_name, lane_states)) in lane_clusters.iter().enumerate() {
        let lane_y = top_margin + lane_idx as f64 * lane_gap;
        lane_y_positions.insert(cluster_name.to_string(), lane_y);

        for st in lane_states {
            let rank = lane_node_ranks.get(st).copied().unwrap_or(0);
            let x = lane_start_x + rank as f64 * rank_step_x;
            let y = lane_y;

            let in_count = total_in_degrees.get(st).copied().unwrap_or(0);
            let out_count = total_out_degrees.get(st).copied().unwrap_or(0);
            let collapsed_badges = collapsed_out_map.get(st).cloned().unwrap_or_default();

            nodes_layout.insert(
                st.clone(),
                NodeLayout {
                    id: st.clone(),
                    label: st.clone(),
                    x,
                    y,
                    width: node_width,
                    height: node_height,
                    is_fault: false,
                    is_initial: false,
                    cluster: cluster_name.to_string(),
                    collapsed_out_badges: collapsed_badges,
                    incoming_count: in_count,
                    outgoing_count: out_count,
                },
            );
        }
    }

    let max_lane_x = lane_start_x + max_rank_in_any_lane as f64 * rank_step_x + node_width;
    let lowest_lane_y = top_margin + lane_clusters.len().saturating_sub(1) as f64 * lane_gap;

    // 4b. Position Entry Hub (IDLE) pinned on left
    let entry_y = (top_margin + lowest_lane_y - lane_gap) * 0.5; // vertically centered among top lanes
    let entry_x = left_margin;
    {
        let in_count = total_in_degrees.get(&entry_hub).copied().unwrap_or(0);
        let out_count = total_out_degrees.get(&entry_hub).copied().unwrap_or(0);
        let collapsed_badges = collapsed_out_map.get(&entry_hub).cloned().unwrap_or_default();
        nodes_layout.insert(
            entry_hub.clone(),
            NodeLayout {
                id: entry_hub.clone(),
                label: entry_hub.clone(),
                x: entry_x,
                y: entry_y,
                width: node_width,
                height: node_height,
                is_fault: false,
                is_initial: true,
                cluster: "INITIAL".to_string(),
                collapsed_out_badges: collapsed_badges,
                incoming_count: in_count,
                outgoing_count: out_count,
            },
        );
    }

    // 4c. Position Exit Hub(s) (RETURNED) pinned on lower-right
    let exit_x = max_lane_x + hub_to_lane_gap;
    let base_exit_y = lowest_lane_y - lane_gap * 0.5; // centered between MOTION and RECOVERY
    for (idx, exit_hub) in exit_hubs.iter().enumerate() {
        let exit_y = base_exit_y + idx as f64 * lane_gap;
        let in_count = total_in_degrees.get(exit_hub).copied().unwrap_or(0);
        let out_count = total_out_degrees.get(exit_hub).copied().unwrap_or(0);
        let collapsed_badges = collapsed_out_map.get(exit_hub).cloned().unwrap_or_default();
        nodes_layout.insert(
            exit_hub.clone(),
            NodeLayout {
                id: exit_hub.clone(),
                label: exit_hub.clone(),
                x: exit_x,
                y: exit_y,
                width: node_width,
                height: node_height,
                is_fault: false,
                is_initial: false,
                cluster: infer_cluster_name(exit_hub, initial_name).to_string(),
                collapsed_out_badges: collapsed_badges,
                incoming_count: in_count,
                outgoing_count: out_count,
            },
        );
    }

    let max_total_op_x = exit_x + node_width;
    let max_total_op_y = lowest_lane_y + node_height;

    // 4d. Position FAULT node(s) pinned centered at bottom
    let fault_y = max_total_op_y + 80.0;
    let center_x = (left_margin + max_total_op_x) * 0.5;
    let fault_states: Vec<String> = sm.states.iter().filter(|s| is_fault_state(s)).cloned().collect();
    let fault_count = fault_states.len().max(1);
    let total_fault_w = fault_count as f64 * node_width + fault_count.saturating_sub(1) as f64 * 40.0;
    let fault_start_x = center_x - total_fault_w * 0.5;

    for (col_idx, node_id) in fault_states.iter().enumerate() {
        let x = fault_start_x + col_idx as f64 * (node_width + 40.0);
        let y = fault_y;
        let in_count = total_in_degrees.get(node_id).copied().unwrap_or(0);
        let out_count = total_out_degrees.get(node_id).copied().unwrap_or(0);
        let collapsed_badges = collapsed_out_map.get(node_id).cloned().unwrap_or_default();
        nodes_layout.insert(
            node_id.clone(),
            NodeLayout {
                id: node_id.clone(),
                label: node_id.clone(),
                x,
                y,
                width: node_width,
                height: node_height,
                is_fault: true,
                is_initial: false,
                cluster: "FAULT".to_string(),
                collapsed_out_badges: collapsed_badges,
                incoming_count: in_count,
                outgoing_count: out_count,
            },
        );
    }

    // Generate cluster lane headers for UI drawing
    let mut lanes = Vec::new();
    for (cluster_name, lane_states) in &lane_clusters {
        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        for st in lane_states {
            if let Some(nl) = nodes_layout.get(st) {
                min_x = min_x.min(nl.x);
                max_x = max_x.max(nl.x + nl.width);
                min_y = min_y.min(nl.y);
            }
        }
        lanes.push(LaneLayout {
            name: cluster_name.to_string(),
            y: min_y,
            x_start: min_x,
            x_end: max_x,
        });
    }
    if !fault_states.is_empty() {
        lanes.push(LaneLayout {
            name: "FAULT".to_string(),
            y: fault_y,
            x_start: fault_start_x,
            x_end: fault_start_x + total_fault_w,
        });
    }
    lanes.sort_by(|a, b| a.y.total_cmp(&b.y));

    // 5. Bézier Edge Routing with Multi-Point Hub Anchors
    let mut edges_layout = Vec::new();

    // Map outgoing edges from entry_hub by target to distribute anchors
    let entry_outgoing: Vec<&AppTransition> = sm
        .transitions
        .iter()
        .filter(|t| t.from == entry_hub && !t.is_fault && !is_fault_state(&t.to))
        .collect();

    let mut overhead_track_idx = 0;
    let mut exit_corridor_track_idx = 0;
    let mut inter_lane_track_map: HashMap<(i32, i32), usize> = HashMap::new();

    for t in &sm.transitions {
        let from_layout = nodes_layout.get(&t.from);
        let to_layout = nodes_layout.get(&t.to);

        if let (Some(from), Some(to)) = (from_layout, to_layout) {
            let display_guard = t.display_guard(35);
            let is_high_fan_in = high_fan_in_nodes.contains(&t.to) && t.from != t.to;
            let is_inter_cluster = from.cluster != to.cluster;

            let (start, end, control1, control2, label_pos, waypoints) = if to.is_fault {
                // Route down toward centered fault
                let start = (from.x + from.width * 0.5, from.y + from.height);
                let end = (to.x + to.width * 0.5, to.y);
                let dy = (end.1 - start.1).max(30.0);
                let control1 = (start.0, start.1 + dy * 0.45);
                let control2 = (end.0, end.1 - dy * 0.45);
                let label_pos = (from.x + from.width * 0.5, from.y + from.height + 24.0);
                let waypoints = if (start.0 - end.0).abs() < 2.0 {
                    vec![start, end]
                } else {
                    let drop_y = lowest_lane_y + node_height + 40.0;
                    vec![start, (start.0, drop_y), (end.0, drop_y), end]
                };
                (start, end, control1, control2, label_pos, waypoints)
            } else if t.from == entry_hub {
                // Outgoing from Entry Hub (IDLE): stack connection anchors along right edge based on target vertical order
                let target_order = entry_outgoing
                    .iter()
                    .position(|x| x.to == t.to && x.guard == t.guard)
                    .unwrap_or(0);
                let n_conns = entry_outgoing.len().max(1);
                let fraction = (target_order as f64 + 1.0) / (n_conns as f64 + 1.0);
                let start_y = from.y + from.height * fraction;
                let start = (from.x + from.width, start_y);
                let dx = (to.x - start.0).max(30.0);
                let control1 = (start.0 + dx * 0.35, start.1);
                let control2 = (to.x - dx * 0.35, to.y + to.height * 0.5);

                let corridor_x = from.x + from.width + 25.0 + (target_order as f64) * 20.0;
                let label_pos = (from.x + from.width + 35.0, start_y - 12.0);
                let (end, waypoints) = if (start.1 - (to.y + to.height * 0.5)).abs() < 2.0 {
                    let end = (to.x, to.y + to.height * 0.5);
                    (end, vec![start, end])
                } else if to.x <= lane_start_x + 10.0 {
                    let end = (to.x, to.y + to.height * 0.5);
                    (end, vec![start, (corridor_x, start.1), (corridor_x, end.1), end])
                } else {
                    let channel_y = to.y - 25.0;
                    let end = (to.x + to.width * 0.5, to.y);
                    (end, vec![start, (corridor_x, start.1), (corridor_x, channel_y), (to.x + to.width * 0.5, channel_y), end])
                };
                (start, end, control1, control2, label_pos, waypoints)
            } else if t.to == entry_hub {
                // Returning to Entry Hub (IDLE): route back smoothly
                if from.y <= to.y {
                    // Coming from upper lane (e.g. CAL_BACKOFF -> IDLE, CLOSING -> IDLE)
                    let start = (from.x + from.width * 0.5, from.y);
                    let end = (to.x + to.width * 0.6, to.y);
                    let arc_y = (top_margin - 30.0 - (overhead_track_idx as f64) * 16.0).min(from.y - 25.0);
                    overhead_track_idx += 1;
                    let control1 = (from.x + from.width * 0.5, arc_y);
                    let control2 = (to.x + to.width * 0.6, arc_y);
                    let label_pos = ((start.0 + end.0) * 0.5, arc_y - 12.0);
                    let waypoints = vec![start, (start.0, arc_y), (end.0, arc_y), end];
                    (start, end, control1, control2, label_pos, waypoints)
                } else {
                    // Coming from lower lane (e.g. OPENING -> IDLE, REC_BACKOFF -> ALIGN_IDLE)
                    let start = (from.x + from.width * 0.5, from.y + from.height);
                    let end = (to.x + to.width * 0.5, to.y + to.height);
                    let route_y = lowest_lane_y + node_height + 30.0;
                    let control1 = ((start.0 + end.0) * 0.5, start.1 + 20.0);
                    let control2 = ((start.0 + end.0) * 0.5, end.1);
                    let label_pos = ((start.0 + end.0) * 0.5, route_y + 14.0);
                    let waypoints = vec![start, (start.0, route_y), (end.0, route_y), end];
                    (start, end, control1, control2, label_pos, waypoints)
                }
            } else if exit_hubs.contains(&t.to) {
                // Entering Exit Hub (RETURNED): direct diagonal from right of lane to left of hub
                let start = (from.x + from.width, from.y + from.height * 0.5);
                let anchor_y = if from.y < to.y {
                    to.y + to.height * 0.3
                } else if from.y > to.y {
                    to.y + to.height * 0.7
                } else {
                    to.y + to.height * 0.5
                };
                let end = (to.x, anchor_y);
                let dx = (end.0 - start.0).max(20.0);
                let control1 = (start.0 + dx * 0.4, start.1);
                let control2 = (end.0 - dx * 0.4, end.1);
                let label_pos = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5 - 14.0);
                let corridor_x = (from.x + from.width + 30.0 + (exit_corridor_track_idx as f64) * 20.0).max((from.x + from.width + to.x) * 0.5);
                exit_corridor_track_idx += 1;
                let waypoints = if (start.1 - end.1).abs() < 2.0 {
                    vec![start, end]
                } else {
                    vec![start, (corridor_x, start.1), (corridor_x, end.1), end]
                };
                (start, end, control1, control2, label_pos, waypoints)
            } else if exit_hubs.contains(&t.from) {
                // Outgoing repeat/loopback from Exit Hub (RETURNED -> GOING, OPENING, CLOSING)
                let start = (from.x + from.width * 0.5, from.y);
                let end = (to.x + to.width * 0.5, to.y);
                let arc_y = (top_margin - 30.0 - (overhead_track_idx as f64) * 16.0).min(from.y.min(to.y) - 25.0);
                overhead_track_idx += 1;
                let control1 = (start.0, arc_y);
                let control2 = (end.0, arc_y);
                let label_pos = ((start.0 + end.0) * 0.5, arc_y - 12.0);
                let waypoints = vec![start, (start.0, arc_y), (end.0, arc_y), end];
                (start, end, control1, control2, label_pos, waypoints)
            } else if (from.y - to.y).abs() < 5.0 {
                // Same horizontal lane
                if from.x + from.width <= to.x + 15.0 {
                    // Forward in same lane
                    let is_adjacent = to.x <= from.x + rank_step_x + 15.0;
                    if is_adjacent {
                        let start = (from.x + from.width, from.y + from.height * 0.5);
                        let end = (to.x, to.y + to.height * 0.5);
                        let dx = (end.0 - start.0).max(20.0);
                        let control1 = (start.0 + dx * 0.4, start.1);
                        let control2 = (end.0 - dx * 0.4, end.1);
                        let label_w = (display_guard.len() as f64 * 6.0 + 10.0).max(28.0);
                        let label_pos = if label_w > 62.0 {
                            ((start.0 + end.0) * 0.5, from.y + from.height + 18.0)
                        } else {
                            ((start.0 + end.0) * 0.5, start.1 + 14.0)
                        };
                        let waypoints = vec![start, end];
                        (start, end, control1, control2, label_pos, waypoints)
                    } else {
                        // Skipping over intermediate node in same lane
                        let start = (from.x + from.width * 0.5, from.y);
                        let channel_y = from.y - 25.0;
                        let end = (to.x + to.width * 0.5, to.y);
                        let control1 = (start.0, channel_y);
                        let control2 = (end.0, channel_y);
                        let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                        let waypoints = vec![start, (start.0, channel_y), (end.0, channel_y), end];
                        (start, end, control1, control2, label_pos, waypoints)
                    }
                } else {
                    // Backward loop within same lane
                    let start = (from.x + from.width * 0.5, from.y);
                    let channel_y = from.y - 25.0;
                    let end = (to.x + to.width * 0.5, to.y);
                    let control1 = (start.0, channel_y);
                    let control2 = (end.0, channel_y);
                    let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                    let waypoints = vec![start, (start.0, channel_y), (end.0, channel_y), end];
                    (start, end, control1, control2, label_pos, waypoints)
                }
            } else {
                // Cross-lane edge (e.g. RETURNING -> RECOVERY)
                let lane1 = (from.y / lane_gap).round() as i32;
                let lane2 = (to.y / lane_gap).round() as i32;
                let pair = (lane1.min(lane2), lane1.max(lane2));
                let track = inter_lane_track_map.entry(pair).or_insert(0);
                let track_offset = (*track as f64) * 16.0 - 8.0;
                *track += 1;

                if from.y < to.y {
                    // Upper lane to lower lane
                    let start = (from.x + from.width * 0.5, from.y + from.height);
                    let end = (to.x + to.width * 0.5, to.y);
                    let base_channel_y = from.y + node_height + (to.y - (from.y + node_height)) * 0.5;
                    let channel_y = base_channel_y + track_offset;
                    let control1 = (start.0, channel_y);
                    let control2 = (end.0, channel_y);
                    let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                    let waypoints = if (start.0 - end.0).abs() < 2.0 {
                        vec![start, end]
                    } else {
                        vec![start, (start.0, channel_y), (end.0, channel_y), end]
                    };
                    (start, end, control1, control2, label_pos, waypoints)
                } else {
                    // Lower lane to upper lane
                    let start = (from.x + from.width * 0.5, from.y);
                    let end = (to.x + to.width * 0.5, to.y + to.height);
                    let base_channel_y = to.y + node_height + (from.y - (to.y + node_height)) * 0.5;
                    let channel_y = base_channel_y + track_offset;
                    let control1 = (start.0, channel_y);
                    let control2 = (end.0, channel_y);
                    let label_pos = ((start.0 + end.0) * 0.5, channel_y - 12.0);
                    let waypoints = if (start.0 - end.0).abs() < 2.0 {
                        vec![start, end]
                    } else {
                        vec![start, (start.0, channel_y), (end.0, channel_y), end]
                    };
                    (start, end, control1, control2, label_pos, waypoints)
                }
            };

            edges_layout.push(EdgeLayout {
                from: t.from.clone(),
                to: t.to.clone(),
                guard: t.guard.clone(),
                label: t.label.clone(),
                display_guard,
                is_fault: t.is_fault,
                start,
                end,
                control1,
                control2,
                waypoints,
                label_pos,
                transition_type: t.transition_type.clone(),
                is_high_fan_in,
                is_inter_cluster,
            });
        }
    }

    // 6. Post-layout label collision avoidance pass
    let mut node_obstacles: HashMap<String, BoundingBox> = HashMap::new();
    for (id, node) in &nodes_layout {
        node_obstacles.insert(id.clone(), BoundingBox {
            x0: node.x - 4.0,
            y0: node.y - 4.0,
            x1: node.x + node.width + 4.0,
            y1: node.y + node.height + 4.0,
        });
    }

    let mut lane_obstacles: Vec<BoundingBox> = Vec::new();
    for lane in &lanes {
        lane_obstacles.push(BoundingBox {
            x0: lane.x_start - 6.0,
            y0: lane.y - 22.0,
            x1: lane.x_start + 220.0,
            y1: lane.y + 2.0,
        });
    }

    let mut placed_label_boxes: Vec<BoundingBox> = Vec::new();

    for edge in &mut edges_layout {
        if edge.display_guard.is_empty() {
            continue;
        }
        let label_w = (edge.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
        let label_h = 20.0;

        let box_at = |cx: f64, cy: f64| BoundingBox {
            x0: cx - label_w * 0.5 - 2.0,
            y0: cy - label_h * 0.5 - 2.0,
            x1: cx + label_w * 0.5 + 2.0,
            y1: cy + label_h * 0.5 + 2.0,
        };

        let is_intra_adjacent = {
            if let (Some(f), Some(t)) = (nodes_layout.get(&edge.from), nodes_layout.get(&edge.to)) {
                (f.y - t.y).abs() < 5.0 && f.x + f.width <= t.x + 15.0 && t.x <= f.x + rank_step_x + 15.0
            } else {
                false
            }
        };

        let (mut cx, mut cy) = edge.label_pos;
        let mut best_box = box_at(cx, cy);

        // For intra-adjacent edges, the label sits right on the horizontal connector between from and to.
        // We exclude from and to node bounding boxes so the label isn't shoved away.
        let collides = |b: &BoundingBox| -> bool {
            for (nid, nobs) in &node_obstacles {
                if is_intra_adjacent && (*nid == edge.from || *nid == edge.to) {
                    continue;
                }
                if b.intersects(nobs) {
                    return true;
                }
            }
            for lobs in &lane_obstacles {
                if b.intersects(lobs) {
                    return true;
                }
            }
            for pl in &placed_label_boxes {
                if b.intersects(pl) {
                    return true;
                }
            }
            false
        };

        if collides(&best_box) {
            let mut resolved = false;
            let mut candidate_offsets = Vec::new();

            for dy in [0.0, 24.0, -24.0, 48.0, -48.0] {
                for dx in [0.0, 50.0, -50.0, 100.0, -100.0, 160.0, -160.0, 230.0, -230.0, 310.0, -310.0] {
                    if dx == 0.0 && dy == 0.0 {
                        continue;
                    }
                    candidate_offsets.push((dx, dy));
                }
            }

            for dy in [70.0, -70.0, 100.0, -100.0, 140.0, -140.0] {
                for dx in [0.0, 60.0, -60.0, 120.0, -120.0, 180.0, -180.0, 260.0, -260.0] {
                    candidate_offsets.push((dx, dy));
                }
            }

            for (dx, dy) in candidate_offsets {
                let cand_box = box_at(cx + dx, cy + dy);
                if !collides(&cand_box) {
                    cx += dx;
                    cy += dy;
                    best_box = cand_box;
                    resolved = true;
                    break;
                }
            }

            if !resolved {
                for step in 1..=30 {
                    let cand_box = box_at(cx + 60.0 * step as f64, cy);
                    if !collides(&cand_box) {
                        cx += 60.0 * step as f64;
                        best_box = cand_box;
                        break;
                    }
                }
            }
        }
        edge.label_pos = (cx, cy);

        placed_label_boxes.push(best_box);
    }

    // 7. Compute true encompassing diagram bounding box
    let mut min_bound_x = left_margin;
    let mut max_bound_x = max_total_op_x;
    let mut min_bound_y = top_margin;
    let mut max_bound_y = fault_y + node_height;

    for node in nodes_layout.values() {
        min_bound_x = min_bound_x.min(node.x);
        max_bound_x = max_bound_x.max(node.x + node.width);
        min_bound_y = min_bound_y.min(node.y);
        max_bound_y = max_bound_y.max(node.y + node.height);
    }
    for lane in &lanes {
        min_bound_x = min_bound_x.min(lane.x_start);
        max_bound_x = max_bound_x.max(lane.x_end);
        min_bound_y = min_bound_y.min(lane.y - 20.0);
    }
    for edge in &edges_layout {
        if !edge.display_guard.is_empty() {
            let label_w = (edge.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
            min_bound_x = min_bound_x.min(edge.label_pos.0 - label_w * 0.5);
            max_bound_x = max_bound_x.max(edge.label_pos.0 + label_w * 0.5);
            min_bound_y = min_bound_y.min(edge.label_pos.1 - 10.0);
            max_bound_y = max_bound_y.max(edge.label_pos.1 + 10.0);
        }
        min_bound_x = min_bound_x.min(edge.control1.0.min(edge.control2.0));
        max_bound_x = max_bound_x.max(edge.control1.0.max(edge.control2.0));
        min_bound_y = min_bound_y.min(edge.control1.1.min(edge.control2.1));
        max_bound_y = max_bound_y.max(edge.control1.1.max(edge.control2.1));
        for pt in &edge.waypoints {
            min_bound_x = min_bound_x.min(pt.0);
            max_bound_x = max_bound_x.max(pt.0);
            min_bound_y = min_bound_y.min(pt.1);
            max_bound_y = max_bound_y.max(pt.1);
        }
    }

    let total_width = (max_bound_x - min_bound_x.min(0.0) + 120.0).max(1300.0);
    let total_height = (max_bound_y - min_bound_y.min(0.0) + 100.0).max(850.0);

    // Strict validation: Ensure analyzer/layout node count matches declared enum variant count
    assert_eq!(
        nodes_layout.len(),
        sm.states.len(),
        "Analyzer node count mismatch: layout generated {} nodes but enum has {} states (missing: {:?})",
        nodes_layout.len(),
        sm.states.len(),
        sm.states.iter().filter(|s| !nodes_layout.contains_key(*s)).collect::<Vec<_>>()
    );

    // Strict validation: Ensure zero coordinate collisions among distinct nodes
    let mut occupied_positions: HashMap<(i64, i64), String> = HashMap::new();
    for (id, node) in &nodes_layout {
        let key = ((node.x * 10.0).round() as i64, (node.y * 10.0).round() as i64);
        if let Some(existing) = occupied_positions.get(&key) {
            panic!(
                "Fatal layout collision: node '{}' and node '{}' share identical coordinates ({:.1}, {:.1})",
                id, existing, node.x, node.y
            );
        }
        occupied_positions.insert(key, id.clone());
    }

    StateMachineLayout {
        nodes: nodes_layout,
        edges: edges_layout,
        lanes,
        width: total_width,
        height: total_height,
    }
}
