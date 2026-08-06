# stm32_03_timers Test Fixture

- **Source Board**: NUCLEO-F446RE
- **Contents**: Contains 5 timer peripherals (`TIM1`, `TIM2`, `TIM3`, `TIM4`, `TIM6`) across 3 distinct HAL usage patterns:
  1. `TIM6`: Base-timer period-elapsed ISR
  2. `TIM2`: Output-compare with a toggle callback on 3 channels
  3. `TIM1`/`TIM3`/`TIM4`: Encoder mode with no ISR
- **Purpose**: Real multi-peripheral, multi-pattern CubeMX project kept specifically to test the graph module's IRQ-to-callback mapping against real generated output. This fixture should not be hand-edited except to simulate specific test scenarios.
