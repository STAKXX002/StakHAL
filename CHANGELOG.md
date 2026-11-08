# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1] - 2026-11-07

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

## [0.4.0] - 2026-11-02

### Added
- Multi-machine application state visualizer with Sugiyama swimlane layout.
- Nucleo board connector pinout inspector with interactive module filtering.
- Traceability build hash insertion and serial monitor integration.
