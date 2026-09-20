# Ground Truth: hatch_deadtime Fixture

## 1. State Machine Discovery Specification

### 1.1 Machine Candidates
- **Expected Candidates Count**: Exactly 1 selectable state machine.
- **Machine Name**: `HatchState (hatchState)`
- **Tracked Variable**: `static HatchState hatchState = HATCH_IDLE;` in `Core/Src/hatch.c`.
- **Excluded Variable**: `static HatchState pendingState = HATCH_IDLE;` in `Core/Src/hatch.c`.
  - Reason: `pendingState` is a staging variable, not an independent state machine. It is written in `hatch_start()` and read inside `hatchState`'s `HATCH_DEADTIME` dispatch block. It is never the subject of its own `switch` or `if/else-if` dispatch logic that assigns back to `pendingState`. Under the tightened discovery criterion, it is excluded from discovery.

### 1.2 States
1. `HATCH_IDLE` (Initial state)
2. `HATCH_DEADTIME`
3. `HATCH_OPENING`
4. `HATCH_CLOSING`

## 2. Transition Specifications

### 2.1 Internal Transitions (inside `hatch_update`)
- `HATCH_DEADTIME -> HATCH_OPENING` [guard: `pendingState == HATCH_OPENING`, label: `pendingState is OPENING`]
- `HATCH_DEADTIME -> HATCH_CLOSING` [guard: `pendingState == HATCH_CLOSING`, label: `pendingState is CLOSING`]
- `HATCH_OPENING -> HATCH_IDLE` [guard: `now - stateStart > OPEN_DURATION_MS`, label: `Timeout (OPEN_DURATION_MS)`]
- `HATCH_CLOSING -> HATCH_IDLE` [guard: `now - stateStart > CLOSE_DURATION_MS`, label: `Timeout (CLOSE_DURATION_MS)`]

### 2.2 Command Transitions (via `commands.c` `commandTable[]`)
- `HATCH_IDLE -> HATCH_DEADTIME` [action: `hatch_open(now)`, label: `CMD: OPEN`]
- `HATCH_IDLE -> HATCH_DEADTIME` [action: `hatch_close(now)`, label: `CMD: CLOSE`]
- `(any state) -> HATCH_IDLE` [action: `hatch_stop_cmd()`, label: `CMD: STOP`]

### 2.3 Forbidden / False Transitions
- **No Self-Loops on Refresh Branches**: Inside `HATCH_OPENING` and `HATCH_CLOSING`, the `else if (now - lastRefresh > HATCH_REFRESH_MS)` branches invoke `hatch_forward()` / `hatch_reverse()` but do NOT reassign `hatchState`. No `HATCH_OPENING -> HATCH_OPENING` or `HATCH_CLOSING -> HATCH_CLOSING` edges may exist.
