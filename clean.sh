#!/bin/bash
# Flight Tracker Pro - Clean build artifacts
# Removes all build artifacts and caches

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}  Flight Tracker Pro - Clean${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

read -p "This will remove all build artifacts and node_modules. Continue? (y/N) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}❌ Cancelled${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🧹 Cleaning project...${NC}"
echo ""

# Remove node_modules
if [ -d "node_modules" ]; then
    echo -e "  • Removing node_modules..."
    rm -rf node_modules/
fi

# Remove frontend build
if [ -d "build" ]; then
    echo -e "  • Removing frontend build..."
    rm -rf build/
fi

# Remove Rust build artifacts
if [ -d "src-tauri/target" ]; then
    echo -e "  • Removing Rust build artifacts..."
    rm -rf src-tauri/target/
fi

# Remove lock files
if [ -f "package-lock.json" ]; then
    echo -e "  • Removing package-lock.json..."
    rm -f package-lock.json
fi

if [ -f "src-tauri/Cargo.lock" ]; then
    echo -e "  • Removing Cargo.lock..."
    rm -f src-tauri/Cargo.lock
fi

echo ""
echo -e "${GREEN}✅ Clean complete!${NC}"
echo ""
echo -e "Run ${YELLOW}./dev.sh${NC} to reinstall dependencies and start developing."
echo ""
