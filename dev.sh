#!/bin/bash
# Flight Tracker Pro - Development Server
# Launches the application in development mode with hot reloading

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Flight Tracker Pro - Dev Server${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}📦 Installing dependencies...${NC}"
    npm install
    echo ""
fi

# Check if Rust dependencies are up to date
echo -e "${GREEN}🔧 Checking Rust dependencies...${NC}"
cd src-tauri
if ! cargo check --quiet 2>/dev/null; then
    echo -e "${YELLOW}⚙️  Building Rust backend...${NC}"
    cargo build
fi
cd ..
echo ""

# Launch Tauri dev server
echo -e "${GREEN}🚀 Launching Flight Tracker Pro...${NC}"
echo -e "${BLUE}Frontend:${NC} http://localhost:5174"
echo -e "${BLUE}Hot reload:${NC} Enabled"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop${NC}"
echo ""

npm run tauri:dev
