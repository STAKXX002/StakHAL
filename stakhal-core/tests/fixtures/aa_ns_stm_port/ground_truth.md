# AA_NS_STM_PORT State Machine — Ground Truth

## 1. Discovered State Machines

The project is modularized across `Core/Src/alignment.c`, `Core/Src/hatch.c`, `Core/Src/system.c`, and `Core/Src/commands.c`.
Two distinct state machines are declared and tracked:

### 1.1 Machine 1: `AlignState` (`state`)
- **File**: `Core/Src/alignment.c`
- **Enum**: `AlignState`
- **Tracked Variable**: `static volatile AlignState state = ALIGN_IDLE;`
- **11 States**:
  1. `ALIGN_IDLE` (Initial)
  2. `CALIBRATING`
  3. `CAL_STOPPING`
  4. `CAL_BACKOFF`
  5. `GOING`
  6. `HOLD`
  7. `RETURNING`
  8. `RETURNED`
  9. `RECOVERY`
  10. `REC_STOPPING`
  11. `REC_BACKOFF`
- **Synthetic Sink Node**: `SYSTEM FAULT` (generated via fault-coordinator calls to `system_fault()`)
- **Action Functions**:
  - `alignment_start_cal()` -> assigns `state = CALIBRATING`
  - `alignment_go()` -> assigns `state = GOING`
  - `alignment_return()` -> assigns `state = RETURNING`
  - `alignment_reset()` -> assigns `state = ALIGN_IDLE`
  - `alignment_emergency_stop()` -> assigns `state = ALIGN_IDLE`
- **Query Functions** (Never treated as transitions):
  - `alignment_is_idle()` -> tests `state == ALIGN_IDLE`
  - `alignment_is_returned()` -> tests `state == RETURNED`
  - `alignment_is_idle_or_returned()` -> tests `state == ALIGN_IDLE || state == RETURNED`
  - `alignment_is_hold()` -> tests `state == HOLD`
  - `alignment_is_calibrated()` -> tests boolean flag `calibrated`

### 1.2 Machine 2: `HatchState` (`hatchState`)
- **File**: `Core/Src/hatch.c`
- **Enum**: `HatchState`
- **Tracked Variable**: `static HatchState hatchState = HATCH_IDLE;`
- **3 States**:
  1. `HATCH_IDLE` (Initial)
  2. `HATCH_OPENING`
  3. `HATCH_CLOSING`
- **Synthetic Sink Node**: NONE (hatch does not invoke `system_fault()`)
- **Action Functions**:
  - `hatch_open()` -> assigns `hatchState = HATCH_OPENING`
  - `hatch_close()` -> assigns `hatchState = HATCH_CLOSING`
  - `hatch_stop_cmd()` -> assigns `hatchState = HATCH_IDLE`
  - `hatch_emergency_stop()` -> assigns `hatchState = HATCH_IDLE`
- **Query Functions**:
  - `hatch_is_busy()` -> tests `hatchState != HATCH_IDLE`

---

## 2. Transition Specifications

### 2.1 AlignState Transitions
- **Internal Transitions** (inside `alignment_update`):
  - `CALIBRATING -> CAL_STOPPING` (`hit[AXIS_Z1] && hit[AXIS_Z2]`)
  - `CAL_STOPPING -> CAL_BACKOFF` (`axes_done() && skewOK()`)
  - `CAL_BACKOFF -> ALIGN_IDLE` (`axes_done()`)
  - `GOING -> HOLD` (`axes_done()`)
  - `RETURNING -> RECOVERY` (`enteringRecovery`)
  - `RETURNING -> RETURNED` (`axes_done()`)
  - `RECOVERY -> REC_STOPPING` (`hit[AXIS_Z1] && hit[AXIS_Z2]`)
  - `REC_STOPPING -> REC_BACKOFF` (`axes_done() && skewOK()`)
  - `REC_BACKOFF -> RETURNED` (`axes_done()`)
- **Fault Transitions** (to synthetic `SYSTEM FAULT` sink):
  - `CALIBRATING -> SYSTEM FAULT` ("CAL TIMEOUT")
  - `CALIBRATING -> SYSTEM FAULT` ("Z2 LIMIT NOT FOUND")
  - `CALIBRATING -> SYSTEM FAULT` ("Z1 LIMIT NOT FOUND")
  - `CAL_STOPPING -> SYSTEM FAULT` ("CAL STOP TIMEOUT")
  - `CAL_STOPPING -> SYSTEM FAULT` ("CAL SKEW")
  - `CAL_BACKOFF -> SYSTEM FAULT` ("CAL BACKOFF TIMEOUT")
  - `GOING -> SYSTEM FAULT` ("GO TIMEOUT")
  - `RETURNING -> SYSTEM FAULT` ("RETURN TIMEOUT")
  - `RECOVERY -> SYSTEM FAULT` ("REC TIMEOUT")
  - `RECOVERY -> SYSTEM FAULT` ("Z2 LIMIT NOT FOUND")
  - `RECOVERY -> SYSTEM FAULT` ("Z1 LIMIT NOT FOUND")
  - `REC_STOPPING -> SYSTEM FAULT` ("REC STOP TIMEOUT")
  - `REC_STOPPING -> SYSTEM FAULT` ("REC SKEW")
  - `REC_BACKOFF -> SYSTEM FAULT` ("REC BACKOFF TIMEOUT")
- **Command Dispatched Transitions** (via `commands.c` `commandTable[]`):
  - `ALIGN_IDLE -> CALIBRATING` [guard: `alignment_is_idle()`, label: `CMD: CAL`]
  - `ALIGN_IDLE -> GOING` [guard: `alignment_is_idle()`, label: `CMD: GO`]
  - `RETURNED -> GOING` [guard: `alignment_is_returned()`, label: `CMD: GO`]
  - `HOLD -> RETURNING` [guard: `alignment_is_hold()`, label: `CMD: RET`]
  - `(any state) -> ALIGN_IDLE` [action: `alignment_reset()`, label: `CMD: RST`]

### 2.2 HatchState Transitions
- **Internal Transitions** (inside `hatch_update`):
  - `HATCH_OPENING -> HATCH_IDLE` (`now - stateStart > OPEN_DURATION_MS`)
  - `HATCH_CLOSING -> HATCH_IDLE` (`now - stateStart > CLOSE_DURATION_MS`)
- **Command Dispatched Transitions** (via `commands.c` `commandTable[]`):
  - `HATCH_IDLE -> HATCH_OPENING` [action: `hatch_open(now)`, label: `CMD: OPEN`]
  - `HATCH_IDLE -> HATCH_CLOSING` [action: `hatch_close(now)`, label: `CMD: CLOSE`]
  - `(any state) -> HATCH_IDLE` [action: `hatch_stop_cmd()`, label: `CMD: STOP`]
