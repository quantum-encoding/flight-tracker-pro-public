#!/bin/bash

# Flight Tracker Pro - Build Script
# Cross-platform build script for Linux, macOS, and Windows

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory (works even when called from another location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Flight Tracker Pro - Build Script${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Detect OS
OS="unknown"
case "$(uname -s)" in
    Linux*)     OS="linux";;
    Darwin*)    OS="macos";;
    MINGW*|MSYS*|CYGWIN*)    OS="windows";;
esac

echo -e "${YELLOW}Detected OS: ${OS}${NC}"
echo ""

# Check for required tools
echo -e "${BLUE}Checking prerequisites...${NC}"

check_command() {
    if command -v "$1" &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} $1 found"
        return 0
    else
        echo -e "  ${RED}✗${NC} $1 not found"
        return 1
    fi
}

MISSING_DEPS=0

check_command "node" || MISSING_DEPS=1
check_command "npm" || MISSING_DEPS=1
check_command "cargo" || MISSING_DEPS=1
check_command "rustc" || MISSING_DEPS=1

if [ $MISSING_DEPS -eq 1 ]; then
    echo ""
    echo -e "${RED}Error: Missing required dependencies.${NC}"
    echo "Please install Node.js, npm, and Rust before building."
    exit 1
fi

echo ""

# Install npm dependencies
echo -e "${BLUE}Installing npm dependencies...${NC}"
npm install

echo ""

# Run type check
echo -e "${BLUE}Running type checks...${NC}"
npm run check || {
    echo -e "${YELLOW}Warning: Type check had warnings (continuing anyway)${NC}"
}

echo ""

# Build mode selection
BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" == "dev" ]; then
    echo -e "${BLUE}Starting development server...${NC}"
    npm run tauri dev
elif [ "$BUILD_MODE" == "release" ]; then
    echo -e "${BLUE}Building release binary...${NC}"
    
    # Run build (may fail on AppImage but DEB/RPM usually succeed)
    npm run tauri build || true

    echo ""
    
    # Check what was actually built
    RELEASE_DIR="$SCRIPT_DIR/src-tauri/target/release"
    BUNDLE_DIR="$RELEASE_DIR/bundle"
    BUILD_SUCCESS=0

    # Check for binary
    if [ -f "$RELEASE_DIR/flight-tracker-pro" ] || [ -f "$RELEASE_DIR/flight-tracker-pro.exe" ]; then
        BUILD_SUCCESS=1
    fi

    if [ $BUILD_SUCCESS -eq 0 ]; then
        echo -e "${RED}========================================${NC}"
        echo -e "${RED}  Build Failed!${NC}"
        echo -e "${RED}========================================${NC}"
        exit 1
    fi

    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  Build Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""

    echo -e "${BLUE}Build outputs:${NC}"
    echo ""

    # Binary
    if [ -f "$RELEASE_DIR/flight-tracker-pro" ]; then
        SIZE=$(du -h "$RELEASE_DIR/flight-tracker-pro" | cut -f1)
        echo -e "  ${GREEN}✓${NC} Binary: $RELEASE_DIR/flight-tracker-pro ($SIZE)"
    elif [ -f "$RELEASE_DIR/flight-tracker-pro.exe" ]; then
        SIZE=$(du -h "$RELEASE_DIR/flight-tracker-pro.exe" | cut -f1)
        echo -e "  ${GREEN}✓${NC} Binary: $RELEASE_DIR/flight-tracker-pro.exe ($SIZE)"
    fi

    # Platform-specific packages
    if [ "$OS" == "linux" ]; then
        # DEB
        DEB_FILE=$(find "$BUNDLE_DIR/deb" -name "*.deb" 2>/dev/null | head -1)
        if [ -n "$DEB_FILE" ] && [ -f "$DEB_FILE" ]; then
            SIZE=$(du -h "$DEB_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} DEB Package: $DEB_FILE ($SIZE)"
        fi

        # RPM
        RPM_FILE=$(find "$BUNDLE_DIR/rpm" -name "*.rpm" 2>/dev/null | head -1)
        if [ -n "$RPM_FILE" ] && [ -f "$RPM_FILE" ]; then
            SIZE=$(du -h "$RPM_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} RPM Package: $RPM_FILE ($SIZE)"
        fi

        # AppImage
        APPIMAGE_FILE=$(find "$BUNDLE_DIR/appimage" -name "*.AppImage" 2>/dev/null | head -1)
        if [ -n "$APPIMAGE_FILE" ] && [ -f "$APPIMAGE_FILE" ]; then
            SIZE=$(du -h "$APPIMAGE_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} AppImage: $APPIMAGE_FILE ($SIZE)"
        else
            echo -e "  ${YELLOW}⚠${NC} AppImage: Not built (linuxdeploy may be missing)"
        fi
    elif [ "$OS" == "macos" ]; then
        # DMG
        DMG_FILE=$(find "$BUNDLE_DIR/dmg" -name "*.dmg" 2>/dev/null | head -1)
        if [ -n "$DMG_FILE" ] && [ -f "$DMG_FILE" ]; then
            SIZE=$(du -h "$DMG_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} DMG: $DMG_FILE ($SIZE)"
        fi

        # App bundle
        APP_FILE=$(find "$BUNDLE_DIR/macos" -name "*.app" -type d 2>/dev/null | head -1)
        if [ -n "$APP_FILE" ] && [ -d "$APP_FILE" ]; then
            SIZE=$(du -sh "$APP_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} App Bundle: $APP_FILE ($SIZE)"
        fi
    elif [ "$OS" == "windows" ]; then
        # MSI
        MSI_FILE=$(find "$BUNDLE_DIR/msi" -name "*.msi" 2>/dev/null | head -1)
        if [ -n "$MSI_FILE" ] && [ -f "$MSI_FILE" ]; then
            SIZE=$(du -h "$MSI_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} MSI Installer: $MSI_FILE ($SIZE)"
        fi

        # NSIS
        NSIS_FILE=$(find "$BUNDLE_DIR/nsis" -name "*.exe" 2>/dev/null | head -1)
        if [ -n "$NSIS_FILE" ] && [ -f "$NSIS_FILE" ]; then
            SIZE=$(du -h "$NSIS_FILE" | cut -f1)
            echo -e "  ${GREEN}✓${NC} NSIS Installer: $NSIS_FILE ($SIZE)"
        fi
    fi

    echo ""
elif [ "$BUILD_MODE" == "clean" ]; then
    echo -e "${BLUE}Cleaning build artifacts...${NC}"
    rm -rf "$SCRIPT_DIR/src-tauri/target"
    rm -rf "$SCRIPT_DIR/node_modules"
    rm -rf "$SCRIPT_DIR/build"
    rm -rf "$SCRIPT_DIR/.svelte-kit"
    echo -e "${GREEN}Clean complete!${NC}"
else
    echo -e "${RED}Unknown build mode: $BUILD_MODE${NC}"
    echo ""
    echo "Usage: ./build.sh [dev|release|clean]"
    echo "  dev     - Start development server with hot reload"
    echo "  release - Build release binaries and packages (default)"
    echo "  clean   - Remove all build artifacts"
    exit 1
fi
