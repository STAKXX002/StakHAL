fn main() {
    let args: Vec<String> = std::env::args().collect();
    match stakhal_ui::cli::parse_cli_args(&args) {
        stakhal_ui::cli::CliAction::ShowHelp => {
            stakhal_ui::cli::print_help();
        }
        stakhal_ui::cli::CliAction::ListTargets => {
            stakhal_ui::cli::print_targets();
        }
        stakhal_ui::cli::CliAction::RunApp { target_override } => {
            if let Some(ref tgt) = target_override {
                std::env::set_var("STAKHAL_TARGET", tgt);
            }
            stakhal_ui::run();
        }
    }
}
