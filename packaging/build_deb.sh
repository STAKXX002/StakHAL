#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PKG_NAME="stakhal"
PKG_VERSION="${1:-$(grep -m 1 '^version =' "${REPO_ROOT}/stakhal-ui/Cargo.toml" | cut -d '"' -f 2 || echo '0.1.0')}"
# Remove leading 'v' if present (e.g. v0.1.0 -> 0.1.0)
PKG_VERSION="${PKG_VERSION#v}"
PKG_ARCH="amd64"
DEB_NAME="${PKG_NAME}_${PKG_VERSION}_${PKG_ARCH}"

echo "============================================================"
echo "[DEB] Building StakHAL Debian Package: ${DEB_NAME}.deb"
echo "============================================================"

# 1. Build optimized release binary
cd "${REPO_ROOT}"
echo "--> Compiling release binary with Cargo..."
cargo build --release -p stakhal-ui

BIN_SRC="${REPO_ROOT}/target/release/stakhal-ui"
if [ ! -f "${BIN_SRC}" ]; then
    echo "[ERROR] Release binary not found at ${BIN_SRC}"
    exit 1
fi

# 2. Prepare staging directory structure
STAGE_DIR="${REPO_ROOT}/target/deb_staging/${DEB_NAME}"
rm -rf "${STAGE_DIR}"
mkdir -p "${STAGE_DIR}/DEBIAN"
mkdir -p "${STAGE_DIR}/usr/bin"
mkdir -p "${STAGE_DIR}/usr/share/applications"
mkdir -p "${STAGE_DIR}/usr/share/icons/hicolor/scalable/apps"
mkdir -p "${STAGE_DIR}/usr/share/doc/${PKG_NAME}"

# 3. Copy binary (installed as `stakhal` on PATH)
cp "${BIN_SRC}" "${STAGE_DIR}/usr/bin/stakhal"
chmod 755 "${STAGE_DIR}/usr/bin/stakhal"

# 4. Copy desktop entry and icon
cp "${SCRIPT_DIR}/stakhal.desktop" "${STAGE_DIR}/usr/share/applications/stakhal.desktop"
chmod 644 "${STAGE_DIR}/usr/share/applications/stakhal.desktop"

cp "${SCRIPT_DIR}/stakhal.svg" "${STAGE_DIR}/usr/share/icons/hicolor/scalable/apps/stakhal.svg"
chmod 644 "${STAGE_DIR}/usr/share/icons/hicolor/scalable/apps/stakhal.svg"

# 5. Copy license & changelog
cp "${REPO_ROOT}/LICENSE" "${STAGE_DIR}/usr/share/doc/${PKG_NAME}/copyright"
chmod 644 "${STAGE_DIR}/usr/share/doc/${PKG_NAME}/copyright"

# 6. Generate DEBIAN/control file
INSTALLED_SIZE=$(du -sk "${STAGE_DIR}/usr" | cut -f1)

cat <<CONTROL > "${STAGE_DIR}/DEBIAN/control"
Package: ${PKG_NAME}
Version: ${PKG_VERSION}
Section: embedded
Priority: optional
Architecture: ${PKG_ARCH}
Installed-Size: ${INSTALLED_SIZE}
Maintainer: STAKXX002 <contact@stakhal.dev>
Depends: libc6 (>= 2.34), libgtk-4-1 (>= 4.12.0), libadwaita-1-0 (>= 1.4.0), stlink-tools, gcc-arm-none-eabi, libnewlib-arm-none-eabi, cmake, ninja-build
Description: Hardware Abstraction Inspector & Firmware Workbench for STM32
 StakHAL is a native developer workbench for STM32 embedded firmware.
 It inspects STM32CubeMX hardware configurations, visualizes application
 state machines with Sugiyama swimlane layouts and pinout connectors, and
 provides an integrated, non-blocking one-click Build & Flash toolchain
 supporting Makefile, CMake, and Ninja projects.
CONTROL

chmod 644 "${STAGE_DIR}/DEBIAN/control"

# 7. Post-install and Post-remove triggers (cache & desktop updates)
cat <<'POSTINST' > "${STAGE_DIR}/DEBIAN/postinst"
#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    if which update-desktop-database >/dev/null 2>&1; then
        update-desktop-database -q /usr/share/applications || true
    fi
    if which gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -q /usr/share/icons/hicolor || true
    fi
fi
exit 0
POSTINST
chmod 755 "${STAGE_DIR}/DEBIAN/postinst"

cat <<'POSTRM' > "${STAGE_DIR}/DEBIAN/postrm"
#!/bin/sh
set -e
if which update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
if which gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
POSTRM
chmod 755 "${STAGE_DIR}/DEBIAN/postrm"

# 8. Build Debian package using dpkg-deb
DIST_DIR="${REPO_ROOT}/dist"
mkdir -p "${DIST_DIR}"
OUTPUT_DEB="${DIST_DIR}/${DEB_NAME}.deb"

echo "--> Packaging with dpkg-deb..."
dpkg-deb --build --root-owner-group "${STAGE_DIR}" "${OUTPUT_DEB}"

echo "============================================================"
echo "[SUCCESS] Generated: ${OUTPUT_DEB}"
ls -lh "${OUTPUT_DEB}"
echo "============================================================"
