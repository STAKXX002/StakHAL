use stakhal_core::target::TargetDefinition;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    RunApp { target_override: Option<String> },
    ListTargets,
    ShowHelp,
}

pub fn parse_cli_args(args: &[String]) -> CliAction {
    let mut target_override = None;
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" | "help" => return CliAction::ShowHelp,
            "--list-targets" => return CliAction::ListTargets,
            "--target" => {
                if i + 1 < args.len() {
                    target_override = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    CliAction::RunApp { target_override }
}

pub fn print_help() {
    println!("StakHAL: Hardware Abstraction Inspector & Toolchain");
    println!();
    println!("Usage:");
    println!("  stakhal-ui [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --target <NAME>     Select firmware target platform (e.g. AA_NS_STM, AA_NS_STM32)");
    println!("  --list-targets      List all supported firmware targets and exit");
    println!("  -h, --help          Show this help text and exit");
    println!();
    println!("Supported Targets:");
    for t in TargetDefinition::all() {
        println!("  {:<14} {}", t.name, t.display_name);
    }
}

pub fn print_targets() {
    println!("Supported StakHAL Targets:");
    for t in TargetDefinition::all() {
        println!(
            "  {:<14} [MCU: {}, Clock: {} MHz, Axes: {}]",
            t.name,
            t.mcu_part,
            t.sysclk_freq_hz / 1_000_000,
            t.num_axes
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cli_args_help() {
        let args = vec!["stakhal-ui".to_string(), "--help".to_string()];
        assert_eq!(parse_cli_args(&args), CliAction::ShowHelp);

        let args = vec!["stakhal-ui".to_string(), "-h".to_string()];
        assert_eq!(parse_cli_args(&args), CliAction::ShowHelp);
    }

    #[test]
    fn test_parse_cli_args_list_targets() {
        let args = vec!["stakhal-ui".to_string(), "--list-targets".to_string()];
        assert_eq!(parse_cli_args(&args), CliAction::ListTargets);
    }

    #[test]
    fn test_parse_cli_args_target_override() {
        let args = vec![
            "stakhal-ui".to_string(),
            "--target".to_string(),
            "AA_NS_STM32".to_string(),
        ];
        assert_eq!(
            parse_cli_args(&args),
            CliAction::RunApp {
                target_override: Some("AA_NS_STM32".to_string())
            }
        );
    }

    #[test]
    fn test_parse_cli_args_default() {
        let args = vec!["stakhal-ui".to_string()];
        assert_eq!(
            parse_cli_args(&args),
            CliAction::RunApp {
                target_override: None
            }
        );
    }
}

