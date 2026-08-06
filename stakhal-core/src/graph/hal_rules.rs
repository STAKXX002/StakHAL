//! Static STM32F4 HAL IRQ handler to HAL callback mapping tables.
//!
//! Known gaps / Peripherals NOT yet covered in this table:
//! - ADC (e.g. ADC_IRQHandler -> HAL_ADC_IRQHandler)
//! - CAN (e.g. CAN1_TX_IRQHandler, CAN1_RX0_IRQHandler, CAN1_RX1_IRQHandler, CAN1_SCE_IRQHandler)
//! - USB (e.g. OTG_FS_IRQHandler, OTG_HS_IRQHandler)
//! - RTC (e.g. RTC_WKUP_IRQHandler, RTC_Alarm_IRQHandler)
//! - DAC, SDIO/SDMMC, FMC, ETH, COMP, OPAMP, RNG, CRC
//! - Non-STM32F4 MCU families (STM32F0, F1, F7, H7, G0, L4, etc.)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalIrqMapping {
    pub irq_handler_name: &'static str,          // e.g. "USART2_IRQHandler"
    pub hal_dispatch_fn: &'static str,            // e.g. "HAL_UART_IRQHandler"
    pub weak_callbacks: &'static [&'static str], // e.g. ["HAL_UART_RxCpltCallback", "HAL_UART_TxCpltCallback"]
    pub peripheral_prefix: &'static str,          // e.g. "USART" — matches ioc::parser prefix
}

pub static HAL_IRQ_MAPPINGS: &[HalIrqMapping] = &[
    // --- USART / UART ---
    HalIrqMapping {
        irq_handler_name: "USART1_IRQHandler",
        hal_dispatch_fn: "HAL_UART_IRQHandler",
        weak_callbacks: &[
            "HAL_UART_TxCpltCallback",
            "HAL_UART_RxCpltCallback",
            "HAL_UART_ErrorCallback",
            "HAL_UART_TxHalfCpltCallback",
            "HAL_UART_RxHalfCpltCallback",
        ],
        peripheral_prefix: "USART",
    },
    HalIrqMapping {
        irq_handler_name: "USART2_IRQHandler",
        hal_dispatch_fn: "HAL_UART_IRQHandler",
        weak_callbacks: &[
            "HAL_UART_TxCpltCallback",
            "HAL_UART_RxCpltCallback",
            "HAL_UART_ErrorCallback",
            "HAL_UART_TxHalfCpltCallback",
            "HAL_UART_RxHalfCpltCallback",
        ],
        peripheral_prefix: "USART",
    },
    HalIrqMapping {
        irq_handler_name: "USART3_IRQHandler",
        hal_dispatch_fn: "HAL_UART_IRQHandler",
        weak_callbacks: &[
            "HAL_UART_TxCpltCallback",
            "HAL_UART_RxCpltCallback",
            "HAL_UART_ErrorCallback",
            "HAL_UART_TxHalfCpltCallback",
            "HAL_UART_RxHalfCpltCallback",
        ],
        peripheral_prefix: "USART",
    },
    HalIrqMapping {
        irq_handler_name: "UART4_IRQHandler",
        hal_dispatch_fn: "HAL_UART_IRQHandler",
        weak_callbacks: &[
            "HAL_UART_TxCpltCallback",
            "HAL_UART_RxCpltCallback",
            "HAL_UART_ErrorCallback",
            "HAL_UART_TxHalfCpltCallback",
            "HAL_UART_RxHalfCpltCallback",
        ],
        peripheral_prefix: "UART",
    },
    HalIrqMapping {
        irq_handler_name: "UART5_IRQHandler",
        hal_dispatch_fn: "HAL_UART_IRQHandler",
        weak_callbacks: &[
            "HAL_UART_TxCpltCallback",
            "HAL_UART_RxCpltCallback",
            "HAL_UART_ErrorCallback",
            "HAL_UART_TxHalfCpltCallback",
            "HAL_UART_RxHalfCpltCallback",
        ],
        peripheral_prefix: "UART",
    },
    HalIrqMapping {
        irq_handler_name: "USART6_IRQHandler",
        hal_dispatch_fn: "HAL_UART_IRQHandler",
        weak_callbacks: &[
            "HAL_UART_TxCpltCallback",
            "HAL_UART_RxCpltCallback",
            "HAL_UART_ErrorCallback",
            "HAL_UART_TxHalfCpltCallback",
            "HAL_UART_RxHalfCpltCallback",
        ],
        peripheral_prefix: "USART",
    },
    // --- GPIO EXTI ---
    HalIrqMapping {
        irq_handler_name: "EXTI0_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    HalIrqMapping {
        irq_handler_name: "EXTI1_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    HalIrqMapping {
        irq_handler_name: "EXTI2_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    HalIrqMapping {
        irq_handler_name: "EXTI3_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    HalIrqMapping {
        irq_handler_name: "EXTI4_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    HalIrqMapping {
        irq_handler_name: "EXTI9_5_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    HalIrqMapping {
        irq_handler_name: "EXTI15_10_IRQHandler",
        hal_dispatch_fn: "HAL_GPIO_EXTI_IRQHandler",
        weak_callbacks: &["HAL_GPIO_EXTI_Callback"],
        peripheral_prefix: "GPIO",
    },
    // --- TIM ---
    HalIrqMapping {
        irq_handler_name: "TIM2_IRQHandler",
        hal_dispatch_fn: "HAL_TIM_IRQHandler",
        weak_callbacks: &[
            "HAL_TIM_PeriodElapsedCallback",
            "HAL_TIM_OC_DelayElapsedCallback",
            "HAL_TIM_IC_CaptureCallback",
            "HAL_TIM_PWM_PulseFinishedCallback",
            "HAL_TIM_TriggerCallback",
            "HAL_TIM_ErrorCallback",
        ],
        peripheral_prefix: "TIM",
    },
    HalIrqMapping {
        irq_handler_name: "TIM3_IRQHandler",
        hal_dispatch_fn: "HAL_TIM_IRQHandler",
        weak_callbacks: &[
            "HAL_TIM_PeriodElapsedCallback",
            "HAL_TIM_OC_DelayElapsedCallback",
            "HAL_TIM_IC_CaptureCallback",
            "HAL_TIM_PWM_PulseFinishedCallback",
            "HAL_TIM_TriggerCallback",
            "HAL_TIM_ErrorCallback",
        ],
        peripheral_prefix: "TIM",
    },
    HalIrqMapping {
        irq_handler_name: "TIM4_IRQHandler",
        hal_dispatch_fn: "HAL_TIM_IRQHandler",
        weak_callbacks: &[
            "HAL_TIM_PeriodElapsedCallback",
            "HAL_TIM_OC_DelayElapsedCallback",
            "HAL_TIM_IC_CaptureCallback",
            "HAL_TIM_PWM_PulseFinishedCallback",
            "HAL_TIM_TriggerCallback",
            "HAL_TIM_ErrorCallback",
        ],
        peripheral_prefix: "TIM",
    },
    HalIrqMapping {
        irq_handler_name: "TIM5_IRQHandler",
        hal_dispatch_fn: "HAL_TIM_IRQHandler",
        weak_callbacks: &[
            "HAL_TIM_PeriodElapsedCallback",
            "HAL_TIM_OC_DelayElapsedCallback",
            "HAL_TIM_IC_CaptureCallback",
            "HAL_TIM_PWM_PulseFinishedCallback",
            "HAL_TIM_TriggerCallback",
            "HAL_TIM_ErrorCallback",
        ],
        peripheral_prefix: "TIM",
    },
    // --- I2C ---
    HalIrqMapping {
        irq_handler_name: "I2C1_EV_IRQHandler",
        hal_dispatch_fn: "HAL_I2C_EV_IRQHandler",
        weak_callbacks: &[
            "HAL_I2C_MasterTxCpltCallback",
            "HAL_I2C_MasterRxCpltCallback",
            "HAL_I2C_SlaveTxCpltCallback",
            "HAL_I2C_SlaveRxCpltCallback",
            "HAL_I2C_AddrCallback",
            "HAL_I2C_ListenCpltCallback",
            "HAL_I2C_MemTxCpltCallback",
            "HAL_I2C_MemRxCpltCallback",
        ],
        peripheral_prefix: "I2C",
    },
    HalIrqMapping {
        irq_handler_name: "I2C1_ER_IRQHandler",
        hal_dispatch_fn: "HAL_I2C_ER_IRQHandler",
        weak_callbacks: &["HAL_I2C_ErrorCallback"],
        peripheral_prefix: "I2C",
    },
    HalIrqMapping {
        irq_handler_name: "I2C2_EV_IRQHandler",
        hal_dispatch_fn: "HAL_I2C_EV_IRQHandler",
        weak_callbacks: &[
            "HAL_I2C_MasterTxCpltCallback",
            "HAL_I2C_MasterRxCpltCallback",
            "HAL_I2C_SlaveTxCpltCallback",
            "HAL_I2C_SlaveRxCpltCallback",
            "HAL_I2C_AddrCallback",
            "HAL_I2C_ListenCpltCallback",
            "HAL_I2C_MemTxCpltCallback",
            "HAL_I2C_MemRxCpltCallback",
        ],
        peripheral_prefix: "I2C",
    },
    HalIrqMapping {
        irq_handler_name: "I2C2_ER_IRQHandler",
        hal_dispatch_fn: "HAL_I2C_ER_IRQHandler",
        weak_callbacks: &["HAL_I2C_ErrorCallback"],
        peripheral_prefix: "I2C",
    },
    HalIrqMapping {
        irq_handler_name: "I2C3_EV_IRQHandler",
        hal_dispatch_fn: "HAL_I2C_EV_IRQHandler",
        weak_callbacks: &[
            "HAL_I2C_MasterTxCpltCallback",
            "HAL_I2C_MasterRxCpltCallback",
            "HAL_I2C_SlaveTxCpltCallback",
            "HAL_I2C_SlaveRxCpltCallback",
            "HAL_I2C_AddrCallback",
            "HAL_I2C_ListenCpltCallback",
            "HAL_I2C_MemTxCpltCallback",
            "HAL_I2C_MemRxCpltCallback",
        ],
        peripheral_prefix: "I2C",
    },
    HalIrqMapping {
        irq_handler_name: "I2C3_ER_IRQHandler",
        hal_dispatch_fn: "HAL_I2C_ER_IRQHandler",
        weak_callbacks: &["HAL_I2C_ErrorCallback"],
        peripheral_prefix: "I2C",
    },
    // --- SPI ---
    HalIrqMapping {
        irq_handler_name: "SPI1_IRQHandler",
        hal_dispatch_fn: "HAL_SPI_IRQHandler",
        weak_callbacks: &[
            "HAL_SPI_TxCpltCallback",
            "HAL_SPI_RxCpltCallback",
            "HAL_SPI_TxRxCpltCallback",
            "HAL_SPI_ErrorCallback",
        ],
        peripheral_prefix: "SPI",
    },
    HalIrqMapping {
        irq_handler_name: "SPI2_IRQHandler",
        hal_dispatch_fn: "HAL_SPI_IRQHandler",
        weak_callbacks: &[
            "HAL_SPI_TxCpltCallback",
            "HAL_SPI_RxCpltCallback",
            "HAL_SPI_TxRxCpltCallback",
            "HAL_SPI_ErrorCallback",
        ],
        peripheral_prefix: "SPI",
    },
    HalIrqMapping {
        irq_handler_name: "SPI3_IRQHandler",
        hal_dispatch_fn: "HAL_SPI_IRQHandler",
        weak_callbacks: &[
            "HAL_SPI_TxCpltCallback",
            "HAL_SPI_RxCpltCallback",
            "HAL_SPI_TxRxCpltCallback",
            "HAL_SPI_ErrorCallback",
        ],
        peripheral_prefix: "SPI",
    },
    // --- DMA Streams ---
    // Note: In STM32 HAL, DMA interrupt handlers invoke function pointers registered per-transfer instance
    // on the DMA_HandleTypeDef struct (XferCpltCallback, XferErrorCallback, etc.) rather than globally named __weak callbacks.
    // As such, weak_callbacks is empty for DMA stream mappings.
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream0_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream1_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream2_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream3_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream4_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream5_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream6_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA1_Stream7_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream0_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream1_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream2_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream3_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream4_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream5_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream6_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
    HalIrqMapping {
        irq_handler_name: "DMA2_Stream7_IRQHandler",
        hal_dispatch_fn: "HAL_DMA_IRQHandler",
        weak_callbacks: &[],
        peripheral_prefix: "DMA",
    },
];

pub fn mappings_for_peripheral_prefix(prefix: &str) -> Vec<&'static HalIrqMapping> {
    HAL_IRQ_MAPPINGS
        .iter()
        .filter(|m| m.peripheral_prefix == prefix)
        .collect()
}

pub fn mapping_for_irq_handler(irq_handler_name: &str) -> Option<&'static HalIrqMapping> {
    HAL_IRQ_MAPPINGS
        .iter()
        .find(|m| m.irq_handler_name == irq_handler_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_usart2() {
        let mapping = mapping_for_irq_handler("USART2_IRQHandler").unwrap();
        assert_eq!(mapping.hal_dispatch_fn, "HAL_UART_IRQHandler");
        assert_eq!(mapping.peripheral_prefix, "USART");
        assert!(mapping.weak_callbacks.contains(&"HAL_UART_RxCpltCallback"));
        assert!(mapping.weak_callbacks.contains(&"HAL_UART_TxCpltCallback"));
    }

    #[test]
    fn test_mappings_for_prefix_tim() {
        let tim_mappings = mappings_for_peripheral_prefix("TIM");
        assert!(!tim_mappings.is_empty());
        assert!(tim_mappings.iter().any(|m| m.irq_handler_name == "TIM2_IRQHandler"));
    }

    #[test]
    fn test_unknown_handler_returns_none() {
        assert!(mapping_for_irq_handler("NotARealHandler").is_none());
    }

    #[test]
    fn test_exti_shared_handlers_distinct() {
        let exti9_5 = mapping_for_irq_handler("EXTI9_5_IRQHandler").unwrap();
        let exti15_10 = mapping_for_irq_handler("EXTI15_10_IRQHandler").unwrap();

        assert_eq!(exti9_5.hal_dispatch_fn, "HAL_GPIO_EXTI_IRQHandler");
        assert_eq!(exti15_10.hal_dispatch_fn, "HAL_GPIO_EXTI_IRQHandler");
        assert_ne!(exti9_5.irq_handler_name, exti15_10.irq_handler_name);
    }
}
