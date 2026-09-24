pub mod builder;
pub mod cross_file;
pub mod discovery;
pub mod labels;
pub mod layout;
pub mod model;
pub mod transition;

pub use builder::*;
pub use discovery::*;
pub use labels::*;
pub use layout::*;
pub use model::*;
pub use transition::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;
    use petgraph::stable_graph::NodeIndex;
    use rust_sugiyama::configure::{Config, CrossingMinimization};

    #[test]
    fn test_discover_docking_firmware_state_machine() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");

        assert_eq!(candidates.len(), 1);
        let sm = &candidates[0];
        assert_eq!(sm.enum_def.name, "SystemState");
        assert_eq!(sm.var.name, "state");
        assert_eq!(sm.var.initial_value, Some("IDLE".to_string()));
        assert_eq!(
            sm.enum_def.variants,
            vec![
                "IDLE",
                "CALIBRATING",
                "CAL_STOPPING",
                "CAL_BACKOFF",
                "GOING",
                "HOLD",
                "RETURNING",
                "RETURNED",
                "RECOVERY",
                "REC_STOPPING",
                "REC_BACKOFF",
                "FAULT",
                "OPENING",
                "CLOSING"
            ]
        );
    }

    #[test]
    fn test_extract_direct_transitions_docking_firmware() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // Print extracted transitions for inspection
        for t in &sm.transitions {
            println!("Direct transition: {} -> {} [guard: '{}']", t.from, t.to, t.guard);
        }
        for a in &sm.ambiguous_transitions {
            println!("Ambiguous assignment: -> {} [guard: '{}', note: '{}']", a.target, a.guard, a.note);
        }

        // Check key direct transitions exist
        assert!(sm.transitions.iter().any(|t| t.from == "CALIBRATING" && t.to == "CAL_STOPPING" && t.guard == "z1Hit && z2Hit"));
        assert!(sm.transitions.iter().any(|t| t.from == "CAL_STOPPING" && t.to == "CAL_BACKOFF" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "CAL_BACKOFF" && t.to == "IDLE" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "GOING" && t.to == "HOLD" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNING" && t.to == "RECOVERY" && t.guard == "enteringRecovery"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNING" && t.to == "RETURNED" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "RECOVERY" && t.to == "REC_STOPPING" && t.guard == "z1Hit && z2Hit"));
        assert!(sm.transitions.iter().any(|t| t.from == "REC_STOPPING" && t.to == "REC_BACKOFF" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "REC_BACKOFF" && t.to == "RETURNED" && t.guard == "axes_done()"));
        assert!(sm.transitions.iter().any(|t| t.from == "OPENING" && t.to == "IDLE" && t.guard == "now - stateStart > OPEN_DURATION_MS"));
        assert!(sm.transitions.iter().any(|t| t.from == "CLOSING" && t.to == "IDLE" && t.guard == "now - stateStart > CLOSE_DURATION_MS"));

        // Check event-triggered transitions from cmd_ready
        assert!(sm.transitions.iter().any(|t| t.from == "IDLE" && t.to == "GOING"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "GOING"));
        assert!(sm.transitions.iter().any(|t| t.from == "HOLD" && t.to == "RETURNING"));
        assert!(sm.transitions.iter().any(|t| t.from == "IDLE" && t.to == "OPENING"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "OPENING"));
        assert!(sm.transitions.iter().any(|t| t.from == "IDLE" && t.to == "CLOSING"));
        assert!(sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "CLOSING"));

        // Check ambiguous transition from RST
        assert!(sm.ambiguous_transitions.iter().any(|a| a.target == "IDLE" && a.guard.contains("RST")));
    }

    #[test]
    fn test_docking_firmware_state_machine_ground_truth_verification() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        assert_eq!(candidates.len(), 1, "expected exactly 1 state machine candidate");

        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // 1. Verify 14 states
        assert_eq!(sm.states.len(), 14, "expected exactly 14 states");
        let expected_states = [
            "IDLE", "CALIBRATING", "CAL_STOPPING", "CAL_BACKOFF", "GOING", "HOLD",
            "RETURNING", "RETURNED", "RECOVERY", "REC_STOPPING", "REC_BACKOFF",
            "FAULT", "OPENING", "CLOSING"
        ];
        for s in &expected_states {
            assert!(sm.states.contains(&s.to_string()), "missing state: {}", s);
        }

        // 2. Verify exactly 31 transitions
        assert_eq!(sm.transitions.len(), 31, "expected 31 transitions, got {}", sm.transitions.len());

        // Check direct transitions
        let expected_direct = [
            ("CALIBRATING", "CAL_STOPPING", "z1Hit && z2Hit"),
            ("CAL_STOPPING", "CAL_BACKOFF", "axes_done()"),
            ("CAL_BACKOFF", "IDLE", "axes_done()"),
            ("GOING", "HOLD", "axes_done()"),
            ("RETURNING", "RECOVERY", "enteringRecovery"),
            ("RETURNING", "RETURNED", "axes_done()"),
            ("RECOVERY", "REC_STOPPING", "z1Hit && z2Hit"),
            ("REC_STOPPING", "REC_BACKOFF", "axes_done()"),
            ("REC_BACKOFF", "RETURNED", "axes_done()"),
            ("OPENING", "IDLE", "now - stateStart > OPEN_DURATION_MS"),
            ("CLOSING", "IDLE", "now - stateStart > CLOSE_DURATION_MS"),
        ];
        for (from, to, guard) in expected_direct {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected direct transition: {} -> {} [{}]", from, to, guard);
        }

        // Check helper transitions (fault and startCal)
        let expected_helpers = [
            ("CALIBRATING", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL TIMEOUT]"),
            ("CALIBRATING", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("CALIBRATING", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("CAL_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: CAL SKEW]"),
            ("CAL_BACKOFF", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL BACKOFF TIMEOUT]"),
            ("GOING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: GO TIMEOUT]"),
            ("RETURNING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: RETURN TIMEOUT]"),
            ("RECOVERY", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC TIMEOUT]"),
            ("RECOVERY", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("RECOVERY", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("REC_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: REC SKEW]"),
            ("REC_BACKOFF", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC BACKOFF TIMEOUT]"),
            ("IDLE", "CALIBRATING", "cmd_ready && strcmp((const char*)rx_buffer, \"CAL\") == 0 [startCal()]"),
        ];
        for (from, to, guard) in expected_helpers {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected helper transition: {} -> {} [{}]", from, to, guard);
        }

        // Check event transitions
        let expected_events = [
            ("IDLE", "GOING"),
            ("RETURNED", "GOING"),
            ("HOLD", "RETURNING"),
            ("IDLE", "OPENING"),
            ("RETURNED", "OPENING"),
            ("IDLE", "CLOSING"),
            ("RETURNED", "CLOSING"),
        ];
        for (from, to) in expected_events {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard.contains("cmd_ready"));
            assert!(found, "missing expected event transition: {} -> {}", from, to);
        }

        // 3. Verify ambiguous transition
        assert_eq!(sm.ambiguous_transitions.len(), 1, "expected exactly 1 ambiguous transition");
        assert_eq!(sm.ambiguous_transitions[0].target, "IDLE");
        assert!(sm.ambiguous_transitions[0].guard.contains("RST"));

        // 4. Verify layout computation
        let layout = compute_state_machine_layout(&sm);
        assert_eq!(layout.nodes.len(), 14);
        assert_eq!(layout.edges.len(), 31);
        assert!(layout.width > 500.0);
        assert!(layout.height > 300.0);

        // Check FAULT node is marked is_fault and has high fan-in
        let fault_node = layout.nodes.get("FAULT").expect("FAULT node layout missing");
        assert!(fault_node.is_fault);
        assert_eq!(fault_node.incoming_count, 12);

        // Check IDLE node is marked is_initial
        let idle_node = layout.nodes.get("IDLE").expect("IDLE node layout missing");
        assert!(idle_node.is_initial);

        // Check cluster assignments
        assert_eq!(layout.nodes.get("CALIBRATING").unwrap().cluster, "CALIBRATION");
        assert_eq!(layout.nodes.get("GOING").unwrap().cluster, "MOTION");
        assert_eq!(layout.nodes.get("RECOVERY").unwrap().cluster, "RECOVERY");
        assert_eq!(layout.nodes.get("OPENING").unwrap().cluster, "MECHANISM");

        // Check collapsed outgoing badges to FAULT
        assert!(layout.nodes.get("CALIBRATING").unwrap().collapsed_out_badges.contains(&"FAULT".to_string()));
        assert!(layout.nodes.get("GOING").unwrap().collapsed_out_badges.contains(&"FAULT".to_string()));

        // Check high fan-in edges
        let high_fan_in_count = layout.edges.iter().filter(|e| e.is_high_fan_in).count();
        assert_eq!(high_fan_in_count, 12, "expected 12 edges targeting FAULT marked is_high_fan_in");

        // 5. Verify exact expected labels for each priority tier
        // Tier 1: Fault edges via fault() helper
        let cal_timeout_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT edge");
        assert_eq!(cal_timeout_edge.label, "CAL TIMEOUT");

        let z2_limit_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("Z2 LIMIT NOT FOUND")).expect("missing Z2 LIMIT edge");
        assert_eq!(z2_limit_edge.label, "Z2 LIMIT NOT FOUND");

        let cal_skew_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "FAULT" && t.guard.contains("CAL SKEW")).expect("missing CAL SKEW edge");
        assert_eq!(cal_skew_edge.label, "CAL SKEW");

        // Tier 2: Command edges
        let cmd_go_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "GOING").expect("missing IDLE->GOING edge");
        assert_eq!(cmd_go_idle.label, "CMD: GO");

        let cmd_ret_hold = sm.transitions.iter().find(|t| t.from == "HOLD" && t.to == "RETURNING").expect("missing HOLD->RETURNING edge");
        assert_eq!(cmd_ret_hold.label, "CMD: RET");

        let cmd_cal_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "CALIBRATING").expect("missing IDLE->CALIBRATING edge");
        assert_eq!(cmd_cal_idle.label, "CMD: CAL");

        // Tier 4: Fallback timeout edge
        let timeout_open_edge = sm.transitions.iter().find(|t| t.from == "OPENING" && t.to == "IDLE").expect("missing OPENING->IDLE edge");
        assert_eq!(timeout_open_edge.label, "Timeout (OPEN_DURATION_MS)");

        let timeout_close_edge = sm.transitions.iter().find(|t| t.from == "CLOSING" && t.to == "IDLE").expect("missing CLOSING->IDLE edge");
        assert_eq!(timeout_close_edge.label, "Timeout (CLOSE_DURATION_MS)");

        // Tier 4: Fallback boolean expressions
        let axes_done_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "CAL_BACKOFF").expect("missing CAL_STOPPING->CAL_BACKOFF edge");
        assert_eq!(axes_done_edge.label, "Axes Done");

        let hits_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "CAL_STOPPING").expect("missing CALIBRATING->CAL_STOPPING edge");
        assert_eq!(hits_edge.label, "Z1 Hit and Z2 Hit");

        let rec_edge = sm.transitions.iter().find(|t| t.from == "RETURNING" && t.to == "RECOVERY").expect("missing RETURNING->RECOVERY edge");
        assert_eq!(rec_edge.label, "Entering Recovery");

        // Negative assertion: explicitly verify printf-only non-transition branches produce 0 edges
        assert!(!sm.transitions.iter().any(|t| t.to == "NO CAL" || t.to == "BUSY" || t.to == "INVALID"), "printf-only branches must not become states");
        assert!(!sm.transitions.iter().any(|t| t.label == "NO CAL" || t.label == "BUSY" || t.label == "INVALID"), "printf-only branches must not produce transition labels");
        assert!(!sm.transitions.iter().any(|t| t.guard.contains("NO CAL") || t.guard.contains("BUSY") || t.guard.contains("INVALID")), "printf-only branches must not appear in transition guards");

        // Verify EdgeLayout fields
        let layout_go = layout.edges.iter().find(|e| e.from == "IDLE" && e.to == "GOING").expect("missing IDLE->GOING in layout");
        assert_eq!(layout_go.label, "CMD: GO");
        assert_eq!(layout_go.display_guard, "CMD: GO");
        assert!(layout_go.guard.contains("strcmp"), "raw guard must be preserved in layout.edges");

        let layout_fault = layout.edges.iter().find(|e| e.from == "CALIBRATING" && e.to == "FAULT" && e.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT layout edge");
        assert_eq!(layout_fault.label, "CAL TIMEOUT");
        assert_eq!(layout_fault.display_guard, "CAL TIMEOUT");
        assert!(layout_fault.guard.contains("fault: CAL TIMEOUT"), "raw guard must be preserved in layout.edges");
    }

    #[test]
    fn test_docking_firmware_v2_state_machine_ground_truth_verification() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware_v2/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        assert_eq!(candidates.len(), 1, "expected exactly 1 state machine candidate");

        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // 1. Verify 14 states (fail loudly if count differs from enum declared variants)
        assert_eq!(sm.states.len(), 14, "expected exactly 14 states");
        assert_eq!(sm.states.len(), candidates[0].enum_def.variants.len());
        let expected_states = [
            "IDLE", "CALIBRATING", "CAL_STOPPING", "CAL_BACKOFF", "GOING", "HOLD",
            "RETURNING", "RETURNED", "RECOVERY", "REC_STOPPING", "REC_BACKOFF",
            "FAULT", "OPENING", "CLOSING"
        ];
        for s in &expected_states {
            assert!(sm.states.contains(&s.to_string()), "missing state: {}", s);
        }

        // 2. Verify exactly 35 transitions
        assert_eq!(sm.transitions.len(), 35, "expected 35 transitions, got {}", sm.transitions.len());

        // Check direct loop transitions (11)
        let expected_direct = [
            ("CALIBRATING", "CAL_STOPPING", "z1Hit && z2Hit"),
            ("CAL_STOPPING", "CAL_BACKOFF", "axes_done()"),
            ("CAL_BACKOFF", "IDLE", "axes_done()"),
            ("GOING", "HOLD", "axes_done()"),
            ("RETURNING", "RECOVERY", "enteringRecovery"),
            ("RETURNING", "RETURNED", "axes_done()"),
            ("RECOVERY", "REC_STOPPING", "z1Hit && z2Hit"),
            ("REC_STOPPING", "REC_BACKOFF", "axes_done()"),
            ("REC_BACKOFF", "RETURNED", "axes_done()"),
            ("OPENING", "IDLE", "now - stateStart > OPEN_DURATION_MS"),
            ("CLOSING", "IDLE", "now - stateStart > CLOSE_DURATION_MS"),
        ];
        for (from, to, guard) in expected_direct {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected direct transition: {} -> {} [{}]", from, to, guard);
        }

        // Check helper transitions (14 fault timeouts/errors + 1 startCal)
        let expected_helpers = [
            ("CALIBRATING", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL TIMEOUT]"),
            ("CALIBRATING", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("CALIBRATING", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("CAL_STOPPING", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL STOP TIMEOUT]"),
            ("CAL_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: CAL SKEW]"),
            ("CAL_BACKOFF", "FAULT", "now - stateStart > CAL_TIMEOUT [fault: CAL BACKOFF TIMEOUT]"),
            ("GOING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: GO TIMEOUT]"),
            ("RETURNING", "FAULT", "now - stateStart > MOVE_TIMEOUT [fault: RETURN TIMEOUT]"),
            ("RECOVERY", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC TIMEOUT]"),
            ("RECOVERY", "FAULT", "z1Hit && !z2Hit && z2.current_pos == z2.target_pos [fault: Z2 LIMIT NOT FOUND]"),
            ("RECOVERY", "FAULT", "z2Hit && !z1Hit && z1.current_pos == z1.target_pos [fault: Z1 LIMIT NOT FOUND]"),
            ("REC_STOPPING", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC STOP TIMEOUT]"),
            ("REC_STOPPING", "FAULT", "axes_done() && !skewOK() [fault: REC SKEW]"),
            ("REC_BACKOFF", "FAULT", "now - stateStart > RECOVERY_TIMEOUT [fault: REC BACKOFF TIMEOUT]"),
            ("IDLE", "CALIBRATING", "cmd_ready && strcmp((const char*)rx_buffer, \"CAL\") == 0 [startCal()]"),
        ];
        for (from, to, guard) in expected_helpers {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard == guard);
            assert!(found, "missing expected helper transition: {} -> {} [{}]", from, to, guard);
        }

        // Check event transitions (including cross-transitions CLOSING -> OPENING and OPENING -> CLOSING)
        let expected_events = [
            ("IDLE", "GOING"),
            ("RETURNED", "GOING"),
            ("HOLD", "RETURNING"),
            ("IDLE", "OPENING"),
            ("RETURNED", "OPENING"),
            ("CLOSING", "OPENING"),
            ("IDLE", "CLOSING"),
            ("RETURNED", "CLOSING"),
            ("OPENING", "CLOSING"),
        ];
        for (from, to) in expected_events {
            let found = sm.transitions.iter().any(|t| t.from == from && t.to == to && t.guard.contains("cmd_ready"));
            assert!(found, "missing expected event transition: {} -> {}", from, to);
        }

        // 3. Verify ambiguous transitions (RST and STOP)
        assert_eq!(sm.ambiguous_transitions.len(), 2, "expected exactly 2 ambiguous transitions");
        assert!(sm.ambiguous_transitions.iter().any(|a| a.target == "IDLE" && a.guard.contains("RST")));
        assert!(sm.ambiguous_transitions.iter().any(|a| a.target == "IDLE" && a.guard.contains("STOP")));

        // 4. Verify layout computation
        let layout = compute_state_machine_layout(&sm);
        assert_eq!(layout.nodes.len(), 14);
        assert_eq!(layout.edges.len(), 35);

        // Verify MECHANISM lane exists and contains OPENING and CLOSING
        let opening = layout.nodes.get("OPENING").expect("OPENING node layout missing");
        let closing = layout.nodes.get("CLOSING").expect("CLOSING node layout missing");
        let returned = layout.nodes.get("RETURNED").expect("RETURNED node layout missing");

        assert_eq!(opening.cluster, "MECHANISM");
        assert_eq!(closing.cluster, "MECHANISM");

        // Verify OPENING and CLOSING are NOT stacked on RETURNED
        assert_ne!((opening.x, opening.y), (returned.x, returned.y), "OPENING must not be stacked on RETURNED");
        assert_ne!((closing.x, closing.y), (returned.x, returned.y), "CLOSING must not be stacked on RETURNED");
        assert_ne!((opening.x, opening.y), (closing.x, closing.y), "OPENING must not be stacked on CLOSING");

        let lane_mechanism = layout.lanes.iter().find(|l| l.name == "MECHANISM");
        assert!(lane_mechanism.is_some(), "MECHANISM flow lane must exist in layout");

        // Check FAULT node
        let fault_node = layout.nodes.get("FAULT").expect("FAULT node layout missing");
        assert!(fault_node.is_fault);
        assert_eq!(fault_node.incoming_count, 14);

        // 5. Verify exact expected labels for each priority tier
        // Tier 1: Fault edges via fault() helper
        let cal_timeout_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT edge");
        assert_eq!(cal_timeout_edge.label, "CAL TIMEOUT");

        let z2_limit_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "FAULT" && t.guard.contains("Z2 LIMIT NOT FOUND")).expect("missing Z2 LIMIT edge");
        assert_eq!(z2_limit_edge.label, "Z2 LIMIT NOT FOUND");

        let cal_skew_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "FAULT" && t.guard.contains("CAL SKEW")).expect("missing CAL SKEW edge");
        assert_eq!(cal_skew_edge.label, "CAL SKEW");

        // Tier 2: Command edges (including cross-transitions)
        let cmd_go_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "GOING").expect("missing IDLE->GOING edge");
        assert_eq!(cmd_go_idle.label, "CMD: GO");

        let cmd_ret_hold = sm.transitions.iter().find(|t| t.from == "HOLD" && t.to == "RETURNING").expect("missing HOLD->RETURNING edge");
        assert_eq!(cmd_ret_hold.label, "CMD: RET");

        let cmd_open_closing = sm.transitions.iter().find(|t| t.from == "CLOSING" && t.to == "OPENING").expect("missing CLOSING->OPENING edge");
        assert_eq!(cmd_open_closing.label, "CMD: OPEN");

        let cmd_close_opening = sm.transitions.iter().find(|t| t.from == "OPENING" && t.to == "CLOSING").expect("missing OPENING->CLOSING edge");
        assert_eq!(cmd_close_opening.label, "CMD: CLOSE");

        let cmd_cal_idle = sm.transitions.iter().find(|t| t.from == "IDLE" && t.to == "CALIBRATING").expect("missing IDLE->CALIBRATING edge");
        assert_eq!(cmd_cal_idle.label, "CMD: CAL");

        // Tier 4: Fallback timeout edge
        let timeout_open_edge = sm.transitions.iter().find(|t| t.from == "OPENING" && t.to == "IDLE").expect("missing OPENING->IDLE edge");
        assert_eq!(timeout_open_edge.label, "Timeout (OPEN_DURATION_MS)");

        let timeout_close_edge = sm.transitions.iter().find(|t| t.from == "CLOSING" && t.to == "IDLE").expect("missing CLOSING->IDLE edge");
        assert_eq!(timeout_close_edge.label, "Timeout (CLOSE_DURATION_MS)");

        // Tier 4: Fallback boolean expressions
        let axes_done_edge = sm.transitions.iter().find(|t| t.from == "CAL_STOPPING" && t.to == "CAL_BACKOFF").expect("missing CAL_STOPPING->CAL_BACKOFF edge");
        assert_eq!(axes_done_edge.label, "Axes Done");

        let hits_edge = sm.transitions.iter().find(|t| t.from == "CALIBRATING" && t.to == "CAL_STOPPING").expect("missing CALIBRATING->CAL_STOPPING edge");
        assert_eq!(hits_edge.label, "Z1 Hit and Z2 Hit");

        let rec_edge = sm.transitions.iter().find(|t| t.from == "RETURNING" && t.to == "RECOVERY").expect("missing RETURNING->RECOVERY edge");
        assert_eq!(rec_edge.label, "Entering Recovery");

        // Negative assertion: explicitly verify printf-only non-transition branches produce 0 edges
        assert!(!sm.transitions.iter().any(|t| t.to == "NO CAL" || t.to == "BUSY" || t.to == "INVALID"), "printf-only branches must not become states");
        assert!(!sm.transitions.iter().any(|t| t.label == "NO CAL" || t.label == "BUSY" || t.label == "INVALID"), "printf-only branches must not produce transition labels");
        assert!(!sm.transitions.iter().any(|t| t.guard.contains("NO CAL") || t.guard.contains("BUSY") || t.guard.contains("INVALID")), "printf-only branches must not appear in transition guards");

        // Verify EdgeLayout fields
        let layout_open = layout.edges.iter().find(|e| e.from == "CLOSING" && e.to == "OPENING").expect("missing CLOSING->OPENING in layout");
        assert_eq!(layout_open.label, "CMD: OPEN");
        assert_eq!(layout_open.display_guard, "CMD: OPEN");
        assert!(layout_open.guard.contains("strcmp"), "raw guard must be preserved in layout.edges");

        let layout_fault = layout.edges.iter().find(|e| e.from == "CALIBRATING" && e.to == "FAULT" && e.guard.contains("CAL_TIMEOUT")).expect("missing CAL_TIMEOUT layout edge");
        assert_eq!(layout_fault.label, "CAL TIMEOUT");
        assert_eq!(layout_fault.display_guard, "CAL TIMEOUT");
        assert!(layout_fault.guard.contains("fault: CAL TIMEOUT"), "raw guard must be preserved in layout.edges");
    }

    #[test]
    fn test_docking_firmware_layout_fixes_round2() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");
        let layout = compute_state_machine_layout(&sm);

        // Fix #1: Within-cluster ordering bug
        // CALIBRATION cluster must render as CALIBRATING -> CAL_STOPPING -> CAL_BACKOFF
        let cal = layout.nodes.get("CALIBRATING").unwrap();
        let cal_stop = layout.nodes.get("CAL_STOPPING").unwrap();
        let cal_back = layout.nodes.get("CAL_BACKOFF").unwrap();
        assert!(cal.x < cal_stop.x, "CALIBRATING.x ({}) must be < CAL_STOPPING.x ({})", cal.x, cal_stop.x);
        assert!(cal_stop.x < cal_back.x, "CAL_STOPPING.x ({}) must be < CAL_BACKOFF.x ({})", cal_stop.x, cal_back.x);

        // RECOVERY cluster must render as RECOVERY -> REC_STOPPING -> REC_BACKOFF
        let rec = layout.nodes.get("RECOVERY").unwrap();
        let rec_stop = layout.nodes.get("REC_STOPPING").unwrap();
        let rec_back = layout.nodes.get("REC_BACKOFF").unwrap();
        assert!(rec.x < rec_stop.x, "RECOVERY.x ({}) must be < REC_STOPPING.x ({})", rec.x, rec_stop.x);
        assert!(rec_stop.x < rec_back.x, "REC_STOPPING.x ({}) must be < REC_BACKOFF.x ({})", rec_stop.x, rec_back.x);

        // Fix #2: Label collision avoidance
        // No two edge labels may overlap, and no edge label may overlap any node
        let labels_with_boxes: Vec<BoundingBox> = layout
            .edges
            .iter()
            .filter(|e| !e.display_guard.is_empty())
            .map(|e| {
                let w = (e.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
                let h = 20.0;
                BoundingBox {
                    x0: e.label_pos.0 - w * 0.5,
                    y0: e.label_pos.1 - h * 0.5,
                    x1: e.label_pos.0 + w * 0.5,
                    y1: e.label_pos.1 + h * 0.5,
                }
            })
            .collect();

        // Check against nodes
        for (idx, (edge, lbox)) in layout.edges.iter().filter(|e| !e.display_guard.is_empty()).zip(labels_with_boxes.iter()).enumerate() {
            for node in layout.nodes.values() {
                let node_box = BoundingBox {
                    x0: node.x,
                    y0: node.y,
                    x1: node.x + node.width,
                    y1: node.y + node.height,
                };
                assert!(
                    !lbox.intersects(&node_box),
                    "Label {} ({} -> {} '{}') at ({}, {}) intersects node {} at ({}, {})",
                    idx, edge.from, edge.to, edge.display_guard, lbox.x0, lbox.y0, node.id, node_box.x0, node_box.y0
                );
            }
        }

        // Check against other labels
        let non_empty_edges: Vec<&EdgeLayout> = layout.edges.iter().filter(|e| !e.display_guard.is_empty()).collect();
        for i in 0..labels_with_boxes.len() {
            for j in (i + 1)..labels_with_boxes.len() {
                assert!(
                    !labels_with_boxes[i].intersects(&labels_with_boxes[j]),
                    "Label {} ({} -> {} '{}' at {:?}) and Label {} ({} -> {} '{}' at {:?}) overlap",
                    i, non_empty_edges[i].from, non_empty_edges[i].to, non_empty_edges[i].display_guard, non_empty_edges[i].label_pos,
                    j, non_empty_edges[j].from, non_empty_edges[j].to, non_empty_edges[j].display_guard, non_empty_edges[j].label_pos
                );
            }
        }

        // Sugiyama forward edge routing: IDLE -> GOING flows left-to-right from IDLE to GOING
        let idle_node = layout.nodes.get("IDLE").unwrap();
        let going_node = layout.nodes.get("GOING").unwrap();
        let idle_to_going = layout.edges.iter().find(|e| e.from == "IDLE" && e.to == "GOING").unwrap();
        assert_eq!(idle_to_going.start.0, idle_node.x + idle_node.width);
        assert_eq!(idle_to_going.end.0, going_node.x);
        assert!(idle_to_going.control1.0 > idle_to_going.start.0);
        assert!(idle_to_going.control2.0 < idle_to_going.end.0);

        // Fix #4: FAULT node placement
        // Centered horizontally under the operational graph, 80px below lowest operational lane
        let fault_node = layout.nodes.get("FAULT").unwrap();
        let max_op_x = layout.nodes.values().filter(|n| !n.is_fault).map(|n| n.x + n.width).fold(110.0, f64::max);
        let center_x = (110.0 + max_op_x) * 0.5;
        let fault_center_x = fault_node.x + fault_node.width * 0.5;
        assert!(
            (fault_center_x - center_x).abs() < 10.0,
            "FAULT node center ({}) should match graph center ({})",
            fault_center_x, center_x
        );

        let max_op_y = layout.nodes.values().filter(|n| !n.is_fault).map(|n| n.y + n.height).fold(80.0, f64::max);
        let fault_gap = fault_node.y - max_op_y;
        assert!(
            (fault_gap - 80.0).abs() < 5.0,
            "FAULT vertical gap ({}) should be ~80px below lowest operational lane",
            fault_gap
        );
    }

    #[test]
    fn test_rust_sugiyama_layout_verification() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");

        // 1. Verify graph construction from extracted state machine data
        let (graph, node_map) = build_state_machine_graph(&sm);
        assert_eq!(graph.node_count(), 14, "expected 14 StateNodes");
        assert_eq!(graph.edge_count(), 19, "expected 19 operational directed edges (12 badge-only FAULT transitions excluded)");

        // 2. Verify connected components structure via rust-sugiyama
        let config = Config {
            vertex_spacing: 100.0,
            c_minimization: CrossingMinimization::Median,
            transpose: true,
            ..Default::default()
        };
        let size_fn = |_idx: NodeIndex, _node: &StateNode| (54.0, 175.0);
        let layouts = rust_sugiyama::from_graph(&graph, &size_fn, &config);

        // As verified, operational states form 1 connected component (IDLE, MOTION, CALIBRATION, RECOVERY, MECHANISM
        // are linked by real transitions in firmware), and FAULT is an isolated single-node component.
        assert_eq!(layouts.len(), 2, "expected 2 components: 13 operational nodes + 1 isolated FAULT");
        assert_eq!(layouts[0].0.len(), 13, "operational component must contain 13 states");
        assert_eq!(layouts[1].0.len(), 1, "fault component must contain 1 state (FAULT)");

        let fault_idx = node_map["FAULT"];
        assert_eq!(layouts[1].0[0].0, fault_idx);

        // 3. Verify rank ordering in full layout
        let layout = compute_state_machine_layout(&sm);

        // CALIBRATING -> CAL_STOPPING -> CAL_BACKOFF strictly ordered left-to-right
        let cal = layout.nodes.get("CALIBRATING").unwrap();
        let cal_stop = layout.nodes.get("CAL_STOPPING").unwrap();
        let cal_back = layout.nodes.get("CAL_BACKOFF").unwrap();
        assert!(cal.x < cal_stop.x, "CALIBRATING.x < CAL_STOPPING.x");
        assert!(cal_stop.x < cal_back.x, "CAL_STOPPING.x < CAL_BACKOFF.x");

        // RECOVERY -> REC_STOPPING -> REC_BACKOFF strictly ordered left-to-right
        let rec = layout.nodes.get("RECOVERY").unwrap();
        let rec_stop = layout.nodes.get("REC_STOPPING").unwrap();
        let rec_back = layout.nodes.get("REC_BACKOFF").unwrap();
        assert!(rec.x < rec_stop.x, "RECOVERY.x < REC_STOPPING.x");
        assert!(rec_stop.x < rec_back.x, "REC_STOPPING.x < REC_BACKOFF.x");

        // Verify collision avoidance
        for edge in layout.edges.iter().filter(|e| !e.display_guard.is_empty()) {
            let label_w = (edge.display_guard.len() as f64 * 6.5 + 16.0).max(36.0);
            let label_h = 20.0;
            let lbox = BoundingBox {
                x0: edge.label_pos.0 - label_w * 0.5,
                y0: edge.label_pos.1 - label_h * 0.5,
                x1: edge.label_pos.0 + label_w * 0.5,
                y1: edge.label_pos.1 + label_h * 0.5,
            };

            for node in layout.nodes.values() {
                let nbox = BoundingBox {
                    x0: node.x,
                    y0: node.y,
                    x1: node.x + node.width,
                    y1: node.y + node.height,
                };
                assert!(!lbox.intersects(&nbox), "label for {} -> {} must not overlap node {}", edge.from, edge.to, node.id);
            }
        }
    }

    #[test]
    fn test_hub_anchored_swimlanes_layout() {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/docking_firmware/Core/Src/main.c");
        let candidates = discover_state_machines_in_file(&fixture_path).expect("failed to discover state machines");
        let sm = extract_state_machine_transitions(&candidates[0], &fixture_path).expect("failed to extract transitions");
        let layout = compute_state_machine_layout(&sm);

        // 1. Hub dynamic identification & positioning
        // IDLE is pinned at left_margin = 110.0
        let idle = layout.nodes.get("IDLE").expect("IDLE node must exist");
        assert_eq!(idle.x, 110.0, "IDLE hub must be pinned at left margin 110.0");

        // RETURNED is pinned at lower-right (x >= 1100.0)
        let ret = layout.nodes.get("RETURNED").expect("RETURNED node must exist");
        assert!(ret.x >= 1100.0, "RETURNED hub must be pinned on the right (x = {})", ret.x);

        // FAULT is centered at bottom
        let fault = layout.nodes.get("FAULT").expect("FAULT node must exist");
        let max_op_x = layout.nodes.values().filter(|n| !n.is_fault).map(|n| n.x + n.width).fold(110.0, f64::max);
        let center_x = (110.0 + max_op_x) * 0.5;
        let fault_center_x = fault.x + fault.width * 0.5;
        assert_eq!(fault_center_x, center_x, "FAULT node center must match graph center");

        // 2. Multi-point connection anchors for IDLE outgoing edges
        let idle_outgoing: Vec<&EdgeLayout> = layout.edges.iter().filter(|e| e.from == "IDLE" && !e.is_fault).collect();
        assert!(idle_outgoing.len() >= 3, "IDLE should have multiple outgoing operational edges");
        let start_ys: HashSet<i64> = idle_outgoing.iter().map(|e| (e.start.1 * 100.0).round() as i64).collect();
        assert!(
            start_ys.len() > 1,
            "IDLE outgoing edges must have multi-point connection anchors along its right edge, found distinct Y count: {}",
            start_ys.len()
        );

        // 3. Lane positioning: all lane states start to the right of IDLE
        for node in layout.nodes.values() {
            if node.id != "IDLE" && !node.is_fault {
                assert!(
                    node.x >= idle.x + idle.width,
                    "Operational node {} at x={} must start to the right of IDLE (x={})",
                    node.id, node.x, idle.x + idle.width
                );
            }
        }

        // 4. Diagram dimensions
        assert!(layout.width >= 1200.0, "diagram width ({}) should encompass hubs and lanes", layout.width);
        assert!(layout.height >= 700.0, "diagram height ({}) should encompass lanes and fault", layout.height);
    }

    #[test]
    fn test_discover_state_machines_in_project_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across project files should succeed");

        assert_eq!(candidates.len(), 2, "Expected exactly 2 state machines (AlignState and HatchState)");

        let align_cand = candidates.iter().find(|c| c.enum_def.name == "AlignState")
            .expect("AlignState candidate must be found");
        assert_eq!(align_cand.var.name, "state");
        assert!(align_cand.var.file_path.ends_with("alignment.c"));
        assert_eq!(align_cand.enum_def.variants.len(), 11);

        let hatch_cand = candidates.iter().find(|c| c.enum_def.name == "HatchState")
            .expect("HatchState candidate must be found");
        assert_eq!(hatch_cand.var.name, "hatchState");
        assert!(hatch_cand.var.file_path.ends_with("hatch.c"));
        assert_eq!(hatch_cand.enum_def.variants.len(), 3);
    }

    #[test]
    fn test_fault_coordinator_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across project files should succeed");

        let align_cand = candidates.iter().find(|c| c.enum_def.name == "AlignState")
            .expect("AlignState candidate must be found");
        let align_file = Path::new(&align_cand.var.file_path);
        let align_sm = extract_state_machine_transitions(align_cand, align_file)
            .expect("AlignState transitions extraction should succeed");

        // AlignState calls system_fault() -> synthetic SYSTEM FAULT sink node must exist
        assert!(align_sm.states.contains(&"SYSTEM FAULT".to_string()), "AlignState must contain synthetic SYSTEM FAULT state");
        assert_eq!(align_sm.states.len(), 12, "Expected 11 enum states + 1 synthetic SYSTEM FAULT state");

        let fault_transitions: Vec<&AppTransition> = align_sm.transitions.iter().filter(|t| t.to == "SYSTEM FAULT").collect();
        assert!(!fault_transitions.is_empty(), "AlignState must have transitions into SYSTEM FAULT");
        for t in &fault_transitions {
            assert!(t.is_fault);
        }

        let fault_labels: Vec<&str> = fault_transitions.iter().map(|t| t.label.as_str()).collect();
        assert!(fault_labels.contains(&"CAL TIMEOUT"));
        assert!(fault_labels.contains(&"CAL SKEW"));
        assert!(fault_labels.contains(&"Z2 LIMIT NOT FOUND"));
        assert!(fault_labels.contains(&"GO TIMEOUT"));
        assert!(fault_labels.contains(&"RETURN TIMEOUT"));
        assert!(fault_labels.contains(&"REC TIMEOUT"));

        // HatchState does NOT call system_fault() -> NO synthetic SYSTEM FAULT node
        let hatch_cand = candidates.iter().find(|c| c.enum_def.name == "HatchState")
            .expect("HatchState candidate must be found");
        let hatch_file = Path::new(&hatch_cand.var.file_path);
        let hatch_sm = extract_state_machine_transitions(hatch_cand, hatch_file)
            .expect("HatchState transitions extraction should succeed");

        assert!(!hatch_sm.states.contains(&"SYSTEM FAULT".to_string()), "HatchState must NOT contain SYSTEM FAULT state");
        assert_eq!(hatch_sm.states.len(), 3, "HatchState must have exactly 3 states");
        assert!(!hatch_sm.transitions.iter().any(|t| t.to == "SYSTEM FAULT" || t.is_fault), "HatchState must have 0 fault transitions");

        // Layout verification: AlignState layout must contain SYSTEM FAULT node
        let align_layout = compute_state_machine_layout(&align_sm);
        let fault_node = align_layout.nodes.get("SYSTEM FAULT").expect("SYSTEM FAULT node layout must exist");
        assert!(fault_node.is_fault);

        // Layout verification: HatchState layout must NOT contain SYSTEM FAULT node
        let hatch_layout = compute_state_machine_layout(&hatch_sm);
        assert!(!hatch_layout.nodes.contains_key("SYSTEM FAULT"), "HatchState layout must NOT have SYSTEM FAULT node");
    }

    #[test]
    fn test_multihop_command_transitions_aa_ns_stm_port() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/aa_ns_stm_port/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across project files should succeed");

        // 1. AlignState multi-hop command transitions
        let align_cand = candidates.iter().find(|c| c.enum_def.name == "AlignState")
            .expect("AlignState candidate must be found");
        let align_file = Path::new(&align_cand.var.file_path);
        let align_sm = extract_state_machine_transitions(align_cand, align_file)
            .expect("AlignState transitions extraction should succeed");

        // Verify CMD: CAL (ALIGN_IDLE -> CALIBRATING)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "ALIGN_IDLE" && t.to == "CALIBRATING" && t.label == "CMD: CAL"),
            "CMD: CAL transition from ALIGN_IDLE to CALIBRATING must exist"
        );

        // Verify CMD: GO (ALIGN_IDLE -> GOING and RETURNED -> GOING)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "ALIGN_IDLE" && t.to == "GOING" && t.label == "CMD: GO"),
            "CMD: GO transition from ALIGN_IDLE to GOING must exist"
        );
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "RETURNED" && t.to == "GOING" && t.label == "CMD: GO"),
            "CMD: GO transition from RETURNED to GOING must exist"
        );

        // Verify CMD: RET (HOLD -> RETURNING)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "HOLD" && t.to == "RETURNING" && t.label == "CMD: RET"),
            "CMD: RET transition from HOLD to RETURNING must exist"
        );

        // Verify CMD: RST ((any state) -> ALIGN_IDLE)
        assert!(
            align_sm.transitions.iter().any(|t| t.from == "(any state)" && t.to == "ALIGN_IDLE" && t.label == "CMD: RST"),
            "CMD: RST transition from (any state) to ALIGN_IDLE must exist"
        );

        // 2. HatchState multi-hop command transitions
        let hatch_cand = candidates.iter().find(|c| c.enum_def.name == "HatchState")
            .expect("HatchState candidate must be found");
        let hatch_file = Path::new(&hatch_cand.var.file_path);
        let hatch_sm = extract_state_machine_transitions(hatch_cand, hatch_file)
            .expect("HatchState transitions extraction should succeed");

        // Verify CMD: OPEN (HATCH_IDLE -> HATCH_OPENING)
        assert!(
            hatch_sm.transitions.iter().any(|t| t.from == "HATCH_IDLE" && t.to == "HATCH_OPENING" && t.label == "CMD: OPEN"),
            "CMD: OPEN transition from HATCH_IDLE to HATCH_OPENING must exist"
        );

        // Verify CMD: CLOSE (HATCH_IDLE -> HATCH_CLOSING)
        assert!(
            hatch_sm.transitions.iter().any(|t| t.from == "HATCH_IDLE" && t.to == "HATCH_CLOSING" && t.label == "CMD: CLOSE"),
            "CMD: CLOSE transition from HATCH_IDLE to HATCH_CLOSING must exist"
        );

        // Verify CMD: STOP ((any state) -> HATCH_IDLE)
        assert!(
            hatch_sm.transitions.iter().any(|t| t.from == "(any state)" && t.to == "HATCH_IDLE" && t.label == "CMD: STOP"),
            "CMD: STOP transition from (any state) to HATCH_IDLE must exist"
        );
    }

    #[test]
    fn test_hatch_deadtime_fixture() {
        let fixture_main_c = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/hatch_deadtime/Core/Src/main.c");
        let candidates = discover_state_machines_in_project(&fixture_main_c)
            .expect("discovery across hatch_deadtime project files should succeed");

        // Ground Truth 1: exactly ONE selectable machine for HatchState (pendingState excluded)
        assert_eq!(candidates.len(), 1, "Expected exactly 1 state machine (hatchState), pendingState must be excluded");
        let cand = &candidates[0];
        assert_eq!(cand.enum_def.name, "HatchState");
        assert_eq!(cand.var.name, "hatchState");
        assert_eq!(cand.display_name, "HatchState (hatchState)");
        assert_eq!(cand.enum_def.variants.len(), 4);

        // Extract transitions
        let hatch_file = Path::new(&cand.var.file_path);
        let sm = extract_state_machine_transitions(cand, hatch_file)
            .expect("HatchState transitions extraction should succeed");

        // Verify states: IDLE, DEADTIME, OPENING, CLOSING
        assert_eq!(sm.states.len(), 4);
        assert!(sm.states.contains(&"HATCH_IDLE".to_string()));
        assert!(sm.states.contains(&"HATCH_DEADTIME".to_string()));
        assert!(sm.states.contains(&"HATCH_OPENING".to_string()));
        assert!(sm.states.contains(&"HATCH_CLOSING".to_string()));

        // Verify HATCH_IDLE -> HATCH_DEADTIME (via commands)
        assert!(
            sm.transitions.iter().any(|t| t.from == "HATCH_IDLE" && t.to == "HATCH_DEADTIME"),
            "Expected transition from HATCH_IDLE to HATCH_DEADTIME"
        );

        // Verify HATCH_DEADTIME -> HATCH_OPENING guarded by pendingState is OPENING
        let deadtime_to_opening = sm.transitions.iter().find(|t| t.from == "HATCH_DEADTIME" && t.to == "HATCH_OPENING")
            .expect("Expected transition from HATCH_DEADTIME to HATCH_OPENING");
        assert!(
            deadtime_to_opening.label == "pendingState is OPENING" || deadtime_to_opening.guard.contains("pendingState"),
            "Expected guard on DEADTIME -> OPENING to be 'pendingState is OPENING', got label: {}, guard: {}",
            deadtime_to_opening.label,
            deadtime_to_opening.guard
        );

        // Verify HATCH_DEADTIME -> HATCH_CLOSING guarded by pendingState is CLOSING
        let deadtime_to_closing = sm.transitions.iter().find(|t| t.from == "HATCH_DEADTIME" && t.to == "HATCH_CLOSING")
            .expect("Expected transition from HATCH_DEADTIME to HATCH_CLOSING");
        assert!(
            deadtime_to_closing.label == "pendingState is CLOSING" || deadtime_to_closing.guard.contains("pendingState"),
            "Expected guard on DEADTIME -> CLOSING to be 'pendingState is CLOSING', got label: {}, guard: {}",
            deadtime_to_closing.label,
            deadtime_to_closing.guard
        );

        // Verify HATCH_OPENING -> HATCH_IDLE and HATCH_CLOSING -> HATCH_IDLE
        assert!(
            sm.transitions.iter().any(|t| t.from == "HATCH_OPENING" && t.to == "HATCH_IDLE"),
            "Expected transition from HATCH_OPENING to HATCH_IDLE"
        );
        assert!(
            sm.transitions.iter().any(|t| t.from == "HATCH_CLOSING" && t.to == "HATCH_IDLE"),
            "Expected transition from HATCH_CLOSING to HATCH_IDLE"
        );

        // Verify NO self-loops on refresh branches
        assert!(
            !sm.transitions.iter().any(|t| t.from == "HATCH_OPENING" && t.to == "HATCH_OPENING"),
            "Spurious self-loop HATCH_OPENING -> HATCH_OPENING must not exist"
        );
        assert!(
            !sm.transitions.iter().any(|t| t.from == "HATCH_CLOSING" && t.to == "HATCH_CLOSING"),
            "Spurious self-loop HATCH_CLOSING -> HATCH_CLOSING must not exist"
        );
    }
}
