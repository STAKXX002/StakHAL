# StakHAL - Hardware Abstraction Inspector & Toolchain

**StakHAL** is a modern, high-performance developer workbench for STM32 embedded firmware engineers. Built natively in **Rust**, **GTK4**, and **Libadwaita**, StakHAL inspects STM32CubeMX projects, visualizes application state machines and physical board pinouts, and provides an integrated, non-blocking **Build & Flash** toolchain.

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
- **Hardware-Accelerated Canvas**: Smooth 60fps pan, zoom, fit-to-view, and click-to-inspect transition details, optimized with batch dot-grid rendering and OpenGL acceleration for WSL2/WSLg.

### 3. Nucleo Physical Pinout Visualizer
- Visual connector inspector for STM32 Nucleo boards (Morpho `CN7`/`CN10` and Arduino Uno `CN5`/`CN6`/`CN8`/`CN9` headers).
- Interactive hover tooltips mapping physical connector pin numbers to MCU GPIO pins and alternate functions.
- Visual conflict detection and warning banners for duplicate pin assignments.

### 4. Integrated Build & Flash Subsystem
- **Multi-Build System Support**: Automatically detects and builds **Makefile**, **CMake**, and **Ninja** projects.
- **Fresh Clone Auto-Configuration**: Detects unconfigured CMake repositories (e.g. freshly cloned from GitHub) and automatically configures presets (`cmake --preset Debug`).
- **Auto `.elf` -> `.bin` Conversion**: Automatically extracts binary flash images using `arm-none-eabi-objcopy` when the build toolchain outputs only `.elf`.
- **ST-Link Probe Auto-Detection**: Scans connected programmers (`st-info --probe`), auto-selects single targets, and presents an interactive picker if multiple ST-Links are connected.
- **Non-Blocking Streaming Console**: Live compiler and flasher output streamed line-by-line into an expandable bottom console drawer with status badges (`BUILDING...`, `PROBING...`, `FLASHING...`, `SUCCESS`).
- Separate **`[ Build ]`** (compile only) and **`[ Build & Flash ]`** (compile + flash to `0x08000000` + hardware reset) actions.

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

> **Windows 11 with WSLg**: When installed inside your Ubuntu WSL2 instance, WSLg automatically publishes StakHAL into the **Windows 11 Start Menu** under `StakHAL (Ubuntu 24.04)`. You can launch it directly from the Windows taskbar with native Wayland graphics acceleration!

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
Install the official Rust toolchain (MSRV: Rust 1.75+):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

---

### Windows 11 Setup

#### Option A: WSL2 with WSLg (Recommended)
Windows 11 natively runs GUI Linux applications with hardware acceleration via **WSLg**:
1. In Windows PowerShell:
   ```powershell
   wsl --install -d Ubuntu-24.04
   ```
2. Inside Ubuntu WSL2, install the Debian package:
   ```bash
   sudo apt install -y ./dist/stakhal_0.1.3_amd64.deb
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
│       ├── toolchain/    # Makefile, CMake, Ninja runners, probe detection, st-flash
│       ├── ui/           # GTK4 components, Main panel, State diagram, Nucleo pinout
│       └── state.rs      # Reactive application state
│
└── Cargo.toml            # Workspace manifest
```

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.


