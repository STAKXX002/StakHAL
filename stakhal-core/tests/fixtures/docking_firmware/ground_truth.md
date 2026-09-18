# Docking Firmware State Machine - Ground Truth

## State Machine Definition
- **Enum Type**: `SystemState`
- **Tracked Variable**: `state`
- **Total States**: 14
- **States**:
  - `IDLE`
  - `CALIBRATING`
  - `CAL_STOPPING`
  - `CAL_BACKOFF`
  - `GOING`
  - `HOLD`
  - `RETURNING`
  - `RETURNED`
  - `RECOVERY`
  - `REC_STOPPING`
  - `REC_BACKOFF`
  - `FAULT`
  - `OPENING`
  - `CLOSING`

---

## Expected Transitions

### 1. Direct Transitions (Loop Logic)
| From | To | Guard Condition |
| :--- | :--- | :--- |
| `CALIBRATING` | `CAL_STOPPING` | `z1Hit && z2Hit` |
| `CAL_STOPPING` | `CAL_BACKOFF` | `axes_done()` |
| `CAL_BACKOFF` | `IDLE` | `axes_done()` |
| `GOING` | `HOLD` | `axes_done()` |
| `RETURNING` | `RECOVERY` | `enteringRecovery` |
| `RETURNING` | `RETURNED` | `axes_done()` |
| `RECOVERY` | `REC_STOPPING` | `z1Hit && z2Hit` |
| `REC_STOPPING` | `REC_BACKOFF` | `axes_done()` |
| `REC_BACKOFF` | `RETURNED` | `axes_done()` |
| `OPENING` | `IDLE` | `now - stateStart > OPEN_DURATION_MS` |
| `CLOSING` | `IDLE` | `now - stateStart > CLOSE_DURATION_MS` |

---

### 2. Event-Triggered Direct Transitions (`cmd_ready` block)
| From | To | Guard Condition / Event Trigger |
| :--- | :--- | :--- |
| `IDLE` | `GOING` | `cmd_ready && strcmp((const char*)rx_buffer, "GO") == 0 && calibrated` |
| `RETURNED` | `GOING` | `cmd_ready && strcmp((const char*)rx_buffer, "GO") == 0 && calibrated` |
| `HOLD` | `RETURNING` | `cmd_ready && strcmp((const char*)rx_buffer, "RET") == 0 && calibrated` |
| `IDLE` | `OPENING` | `cmd_ready && strcmp((const char*)rx_buffer, "OPEN") == 0` |
| `RETURNED` | `OPENING` | `cmd_ready && strcmp((const char*)rx_buffer, "OPEN") == 0` |
| `IDLE` | `CLOSING` | `cmd_ready && strcmp((const char*)rx_buffer, "CLOSE") == 0` |
| `RETURNED` | `CLOSING` | `cmd_ready && strcmp((const char*)rx_buffer, "CLOSE") == 0` |

---

### 3. Indirect Transitions via Helper Functions
| From | To | Helper Call & Argument | Enclosing Guard Condition |
| :--- | :--- | :--- | :--- |
| `CALIBRATING` | `FAULT` | `fault("CAL TIMEOUT")` | `now - stateStart > CAL_TIMEOUT` |
| `CALIBRATING` | `FAULT` | `fault("Z2 LIMIT NOT FOUND")` | `z1Hit && !z2Hit && (z2.current_pos == z2.target_pos)` |
| `CALIBRATING` | `FAULT` | `fault("Z1 LIMIT NOT FOUND")` | `z2Hit && !z1Hit && (z1.current_pos == z1.target_pos)` |
| `CAL_STOPPING` | `FAULT` | `fault("CAL SKEW")` | `axes_done() && !skewOK()` |
| `CAL_BACKOFF` | `FAULT` | `fault("CAL BACKOFF TIMEOUT")` | `now - stateStart > CAL_TIMEOUT` |
| `GOING` | `FAULT` | `fault("GO TIMEOUT")` | `now - stateStart > MOVE_TIMEOUT` |
| `RETURNING` | `FAULT` | `fault("RETURN TIMEOUT")` | `now - stateStart > MOVE_TIMEOUT` |
| `RECOVERY` | `FAULT` | `fault("REC TIMEOUT")` | `now - stateStart > RECOVERY_TIMEOUT` |
| `RECOVERY` | `FAULT` | `fault("Z2 LIMIT NOT FOUND")` | `z1Hit && !z2Hit && (z2.current_pos == z2.target_pos)` |
| `RECOVERY` | `FAULT` | `fault("Z1 LIMIT NOT FOUND")` | `z2Hit && !z1Hit && (z1.current_pos == z1.target_pos)` |
| `REC_STOPPING` | `FAULT` | `fault("REC SKEW")` | `axes_done() && !skewOK()` |
| `REC_BACKOFF` | `FAULT` | `fault("REC BACKOFF TIMEOUT")` | `now - stateStart > RECOVERY_TIMEOUT` |
| `IDLE` | `CALIBRATING` | `startCal()` | `cmd_ready && strcmp((const char*)rx_buffer, "CAL") == 0` |

---

### 4. Ambiguous / Unresolved Cases
| From | To | Context / Trigger | Note |
| :--- | :--- | :--- | :--- |
| `(any state)` | `IDLE` | `cmd_ready && strcmp((const char*)rx_buffer, "RST") == 0` | Unconditional assignment to `state` outside any enclosing `state == X` guard |
