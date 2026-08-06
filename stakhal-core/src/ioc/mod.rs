pub mod discovery;
pub mod parser;

pub use discovery::{discover_project_files, DiscoveryError};
pub use parser::{
    parse_ioc, parse_ioc_str, IocParseError, IocProject, PeripheralConfig, PinConfig,
};
