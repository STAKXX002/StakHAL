# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1] - 2026-11-08

### Added
- Support for target codebase AA_NS_STM32 (4-axis STM32F446 platform).
- Target abstraction trait and struct in stakhal-core::target.
- Automatic target detection from project directory layout and build system artifacts.
- Command-line interface flags --target <NAME>, --list-targets, and --help in stakhal-ui.
- Target-specific stakhal_build_info.h template rendering with backward-compatible macro support (AA_BUILD_HASH and STAKHAL_BUILD_HASH).
- Target configuration in AppConfig schema with field-identifying validation error reporting.
- Serial monitor boot banner matching for both STAKHAL_BUILD: and AA_BUILD: streams.
- Golden-file and regression guard integration tests.

### Changed
- Refactored build info header generation out of inline strings into the target module.
- README documents the verified toolchain (Rust 1.97.1). No minimum supported Rust version is declared.

### Fixed
- Cargo.lock was out of sync with the 0.4.0 manifests, which broke cargo build --locked.
- Clippy lints that fail -D warnings on current toolchains, and rustfmt drift across the workspace.
- Tests no longer depend on machine-specific paths. Tests that use external projects are enabled through STAKHAL_TEST_AA_NS_STM_PORT, STAKHAL_TEST_AA_NS_STM32 and STAKHAL_TEST_CMAKE_CONFIGURE.
- The .ioc parser dropped pins whose CubeMX name carries a function suffix (PA0-WKUP, PH0-OSC_IN, PH1-OSC_OUT).
- Pin-to-module detection matches adjacent (GPIOx, GPIO_PIN_n) pairs in source. Pins addressed through raw port and pin tables are now assigned to their module, and unrelated pins are no longer assigned because the port and pin identifiers appear separately in a file.
- IWDG and WWDG are listed as peripherals, the CubeMX GPIO.groupedBy UI key no longer appears as a GPIO peripheral, and peripheral mode is derived from pin-level mode keys.

## [0.4.0] - 2026-11-02

### Added
- Multi-machine application state visualizer with Sugiyama swimlane layout.
- Nucleo board connector pinout inspector with interactive module filtering.
- Traceability build hash insertion and serial monitor integration.
