# StakHAL - Hardware Abstraction Inspector & Toolchain

**StakHAL** is a desktop tool for inspecting and developing STM32CubeMX firmware projects. Written in **Rust**, **GTK4**, and **Libadwaita**, StakHAL parses CubeMX configurations, visualizes application state machines and Nucleo board pinouts, and includes an integrated build, flash, and serial monitoring workflow.

> **Project Status & Limitations**: StakHAL's state-machine and command-table analysis is pattern-based and has been validated against a small number of real firmware projects so far. It may not correctly detect every C coding style or firmware architecture. Unresolved or ambiguous transitions are surfaced directly in the interface rather than guessed at.

---

## Features

### 1. CubeMX Project Inspection
- Automatically discovers and parses CubeMX `.ioc` hardware configuration files and C source files across project directories (`Core/Src`, `Src`, etc.).
- Extracts MCU family (`STM32F4`, etc.), chip part number, and peripheral configurations (GPIO, TIM, USART, etc.).
- Identifies and validates `USER CODE BEGIN` / `USER CODE END` code blocks and main `while (1)` loop boundaries.

### 2. Multi-Machine Application State Visualizer
- **Multi-File State Machine Discovery**: Automatically discovers state machines across modular multi-file architectures (`alignment.c`, `hatch.c`, etc.) as well as monolithic single-file projects (`main.c`).
- **Interactive Multi-Machine Dropdown**: Switch seamlessly between individual discovered machines (e.g. `AlignState`, `HatchState`) with dedicated per-machine layout computation.
- **Cross-Cutting Fault Coordinator Modeling**: Detects external fault coordinators (`system_fault()`, etc.) and automatically injects a synthetic `"SYSTEM FAULT"` sink node with targeted fault transition edges.
- **Multi-Hop Command Table Dispatch**: Traces serial command tables (`commandTable[]`) mapping CLI string tokens (`"GO"`, `"CAL"`, `"RET"`, `"OPEN"`, `"CLOSE"`) through handlers and query function guards (`alignment_is_idle()`) to state assignments (`alignment_go()`).
- **Hub-Anchored Swimlane Layout**: Uses a customized Sugiyama algorithm (`rust-sugiyama`) anchored around high-degree hub states with dedicated parallel swimlanes for functional clusters.
- **Human-Readable Transition Badges**: Prioritizes fault string literals, command triggers (`CMD: GO`, `CMD: RET`), timeout expressions, and clean boolean guards.
- **Interactive Canvas**: Pan, zoom, fit-to-view, and click-to-inspect transition details on a dot-grid canvas.

### 3. Nucleo Physical Pinout Visualizer
- **Header & Pin Inspection**: Visual connector inspector for STM32 Nucleo boards (Morpho `CN7`/`CN10` and Arduino Uno `CN5`/`CN6`/`CN8`/`CN9` headers).
- **Module Filter Dropdown**: Filter pins by discovered firmware module (`All Modules`, `hatch`, `alignment`, etc.) to isolate subsystem pin usage.
- **Module-Aware Pin Muting**: Visually separates pins into "Active in Module" (full badge fill and bright accent stroke), "Active Elsewhere in Project" (neutral fill with muted accent outline), and unused pins.
- **Hover Details & Tooltips**: Interactive hover tooltips mapping physical connector pin numbers to MCU GPIO names, assigned labels, and alternate functions.
- **Conflict Detection**: Warning banners and visual highlighting for duplicate or conflicting pin assignments.

### 4. Integrated Build & Flash Subsystem
- **Multi-Build System Support**: Automatically detects and builds **Makefile**, **CMake**, and **Ninja** projects.
- **Fresh Clone Auto-Configuration**: Detects unconfigured CMake repositories (e.g. freshly cloned from GitHub) and automatically configures presets (`cmake --preset Debug`).
- **Auto `.elf` -> `.bin` Conversion**: Automatically extracts binary flash images using `arm-none-eabi-objcopy` when the build toolchain outputs only `.elf`.
- **ST-Link Probe Auto-Detection**: Scans connected programmers (`st-info --probe`), auto-selects single targets, and presents an interactive picker if multiple ST-Links are connected.
- **Non-Blocking Streaming Console**: Live compiler and flasher output streamed line-by-line into an expandable bottom console drawer with status badges (`BUILDING...`, `PROBING...`, `FLASHING...`, `SUCCESS`).
- Separate **`[ Build ]`** (compile only) and **`[ Build & Flash ]`** (compile + flash to `0x08000000` + hardware reset) actions.

### 5. Serial Monitor & Command Console
- **Live UART Receive Display**: Streams incoming serial data into a line-buffered log panel with CRLF normalization, auto-scroll, and partial-line timeout flushing.
- **Baud Rate Auto-Detection**: Traces `_write()` / `_io_putchar` printf retargeting to the active console UART (`huart2`, etc.) and automatically parses `huart<N>.Init.BaudRate = <value>;` from source, with manual dropdown override.
- **Port Enumeration & Connection**: Discovers available ST-Link virtual COM ports (e.g. `/dev/ttyACM0`), auto-selects single connected boards, and provides connection status indicators (`CONNECTED`, `DISCONNECTED`, `RECONNECTING...`).
- **Command Transmission with History**: Free-text command entry supporting click and Enter keypress, automatically appending `\r\n` line termination, with Up/Down arrow key history navigation across past commands.
- **Dynamic Quick-Send Buttons**: Surfaces one-click action buttons generated directly from discovered firmware `commandTable` entries (`GO`, `CAL`, `STOP`, `RET`, etc.) when present.
- **Build -> Flash -> Observe Loop**: Automatically switches to the Serial Monitor view on successful flash and retries port reconnection across target reset and USB re-enumeration.

---

## Installation & Setup

### Method 1: Debian Package (`.deb`) - Recommended

The easiest way to install StakHAL on **Ubuntu 24.04 LTS** or **Windows 11 (via WSL2 / WSLg)**. The package automatically installs the desktop application, system icons, and all required embedded toolchain dependencies (`stlink-tools`, `gcc-arm-none-eabi`, `cmake`, `ninja-build`, `libadwaita-1`).

#### 1. Download & Install the Package
Download the latest `stakhal_*.deb` from [GitHub Releases](https://github.com/STAKXX002/StakHAL/releases), then install with:
```bash
sudo apt update
sudo apt install -y ./stakhal_*_amd64.deb
```

> **Windows 11 with WSLg**: When installed inside your Ubuntu WSL2 instance, WSLg automatically publishes StakHAL into the **Windows 11 Start Menu** under `StakHAL (Ubuntu 24.04)`. You can launch it directly from the Windows taskbar.

#### 2. Building the `.deb` Package from Source
To create your own standalone Debian package:
```bash
./packaging/build_deb.sh
```
The output `.deb` package will be generated under `dist/stakhal_<version>_amd64.deb`.

---

### Method 2: Building from Source (Developer Setup)

#### Ubuntu 24.04 LTS

##### 1. System & GUI Dependencies
```bash
sudo apt update
sudo apt install -y \
    build-essential \
    libgtk-4-dev \
    libadwaita-1-dev
```

##### 2. Embedded Toolchain & Flashing Tools
```bash
sudo apt install -y \
    cmake \
    ninja-build \
    gcc-arm-none-eabi \
    libnewlib-arm-none-eabi \
    stlink-tools
```

> **Note on USB permissions**: If flashing without `sudo`, ensure your user has access to ST-Link USB devices by installing udev rules (included with `stlink-tools` or under `/etc/udev/rules.d/49-stlinkv*.rules`).

##### 3. Install Rust Toolchain
Install the official Rust toolchain:
- Verified toolchain: Rust 1.97.1 (stable channel). A minimum supported Rust version has not been established.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

---

### Windows 11 Setup

#### Option A: WSL2 with WSLg (Recommended)
Windows 11 supports graphical Linux applications via **WSLg**:
1. In Windows PowerShell:
   ```powershell
   wsl --install -d Ubuntu-24.04
   ```
2. Inside Ubuntu WSL2, install the Debian package:
   ```bash
   sudo apt install -y ./dist/stakhal_*_amd64.deb
   ```
3. StakHAL will now appear in your **Windows 11 Start Menu** and can be launched directly from Windows.
4. To enable ST-Link USB hardware access inside WSL2, install [usbipd-win](https://github.com/dorssel/usbipd-win):
   ```powershell
   usbipd list
   usbipd bind --busid <bus-id>
   usbipd attach --wsl --busid <bus-id>
   ```

#### Option B: Native Windows (MSYS2 / MinGW-w64)
1. Install [MSYS2](https://www.msys2.org/).
2. Open the **UCRT64** terminal and install dependencies:
   ```bash
   pacman -S mingw-w64-ucrt-x86_64-gtk4 \
             mingw-w64-ucrt-x86_64-libadwaita \
             mingw-w64-ucrt-x86_64-rust \
             mingw-w64-ucrt-x86_64-arm-none-eabi-gcc \
             mingw-w64-ucrt-x86_64-cmake \
             mingw-w64-ucrt-x86_64-ninja
   ```
3. Install [ST-Link Tools for Windows](https://github.com/stlink-org/stlink/releases) and ensure `st-flash.exe` and `st-info.exe` are on `PATH`.

---

## Quick Start

### 1. Clone the Repository
```bash
git clone https://github.com/STAKXX002/StakHAL.git
cd StakHAL
```

### 2. Run All Tests
```bash
cargo test --workspace
```

### 3. Launch StakHAL UI
```bash
cargo run --release -p stakhal-ui
```

---

## Target Platforms

StakHAL provides target definitions to configure hardware parameters, build traceability header fields, and serial protocol prefixes for supported firmware repositories.

### Supported Targets

| Target Identifier | Platform Description | MCU Part | Clock | Axes | Serial Prefix | Header Template |
|---|---|---|---|---|---|---|
| `AA_NS_STM` | Legacy Active Alignment port | STM32F446RET6 | 180 MHz | 2 | `STAKHAL_BUILD:` | Minimal `STAKHAL_BUILD_HASH` |
| `AA_NS_STM32` | 4-Axis Active Alignment platform | STM32F446RET6 | 180 MHz | 4 | `AA_BUILD:` | Extended target configuration macros |

### Automatic Target Detection

When a project directory is selected, StakHAL inspects the file tree to determine the appropriate target definition:

1. `AA_NS_STM32` is identified when any of the following signatures are detected:
   - Presence of `AA_STM.ioc` in the project root.
   - Presence of `Core/Inc/aa_config.h`.
   - `CMakeLists.txt` declaring project or executable name `AA_STM`.
2. `AA_NS_STM` is selected when `aa_ns_stm_port.ioc` or `AA_NS_STM_V1` declarations are detected.
3. Fallback defaults to `AA_NS_STM` for generic or unclassified STM32CubeMX projects.

### Command-Line Target Selection

Target selection can be queried or overridden directly from the command line:

```bash
# List all registered firmware targets
cargo run --release -p stakhal-ui -- --list-targets

# Launch StakHAL configured explicitly for AA_NS_STM32
cargo run --release -p stakhal-ui -- --target AA_NS_STM32

# Display CLI help text
cargo run --release -p stakhal-ui -- --help
```

### Traceability Header Generation for AA_NS_STM32

For the `AA_NS_STM32` target, StakHAL generates `Core/Inc/stakhal_build_info.h` with the following configuration fields:

```c
/* Generated automatically by StakHAL. Do not edit or commit. */
#ifndef STAKHAL_BUILD_INFO_H
#define STAKHAL_BUILD_INFO_H

#define STAKHAL_TARGET_NAME "AA_NS_STM32"
#define STAKHAL_BUILD_HASH "<git-hash>"
#define AA_BUILD_HASH "<git-hash>"
#define STAKHAL_MCU_FAMILY "STM32F4"
#define STAKHAL_MCU_PART "STM32F446RET6"
#define STAKHAL_SYSCLK_FREQ_HZ 180000000UL
#define STAKHAL_NUM_AXES 4

#endif /* STAKHAL_BUILD_INFO_H */
```

Both `STAKHAL_BUILD_HASH` and `AA_BUILD_HASH` macros are defined to ensure compatibility with both internal traceability tools and the `printf("AA_BUILD: %s\r\n", AA_BUILD_HASH);` boot banner in `AA_NS_STM32`.

---

## Architecture

The project is structured as a Cargo workspace:

```
StakHAL/
├── stakhal-core/         # Pure Rust headless core engine
│   ├── src/
│   │   ├── ir/           # Common data models and project schema
│   │   ├── ioc/          # CubeMX .ioc discovery and property parser
│   │   ├── source/       # C AST marker scanner, PV extraction, usage finder
│   │   ├── graph/        # State machine discovery, Sugiyama & swimlane layout
│   │   └── nucleo_pinout/# STM32 Nucleo board connector pin mapping tables
│   └── tests/            # Integration and regression test fixtures
│
├── stakhal-ui/           # Native GTK4 / Libadwaita desktop application
│   └── src/
│       ├── toolchain/    # Makefile, CMake, Ninja runners, probe detection, st-flash, serial port
│       ├── ui/           # Main panel, State diagram, Nucleo pinout, Serial monitor
│       └── state.rs      # Reactive application state
│
└── Cargo.toml            # Workspace manifest
```

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.


