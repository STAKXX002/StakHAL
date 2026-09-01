# StakHAL

StakHAL is a hardware abstraction layer and analysis tool.

Ubuntu 24.04 only, under active development

## Frontends

- **stakhal-qt** (Primary): Modern PyQt6 frontend built on PyO3 `stakhal-py` bindings.
- **stakhal-ui** (Fallback / Legacy): Native GTK4 frontend.

## Development

- **Verify Scan Tool**: Run `cargo run -p stakhal-core --example verify_scan -- <path-to-ioc> <path-to-main.c>` to manually inspect `.ioc` parsing and C source user-region marker scanning against a CubeMX project.


