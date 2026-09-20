pub mod console_uart;
pub mod marker_scan;
pub mod pin_modules;
pub mod pv_extract;
pub mod render_model;
pub mod usage_finder;
pub mod writeback;

pub use console_uart::{detect_console_uart, ConsoleUartInfo};
pub use marker_scan::{
    find_loop_body_gap, is_byte_in_user_region, scan_file, scan_source, ScanError, UserRegion,
};
pub use pin_modules::{discover_module_files, is_infra_file, scan_pin_modules};
pub use pv_extract::{extract_pv_declarations, PvDeclaration, PvExtractError};
pub use render_model::{build_source_render_model, LineTier, RenderedLine};
pub use usage_finder::{find_variable_usages, UsageSite};
pub use writeback::{write_region, WritebackError};


