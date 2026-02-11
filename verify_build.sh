#!/bin/bash
# Verification script for Meetily speaker diarization build

set -e

echo "🔍 Verifying Meetily Build with Speaker Diarization"
echo "=================================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if build artifacts exist
echo "📦 Checking build artifacts..."
if [ -f "target/release/bundle/deb/meetily_0.2.0_amd64.deb" ]; then
    SIZE=$(du -h target/release/bundle/deb/meetily_0.2.0_amd64.deb | cut -f1)
    echo -e "${GREEN}✓${NC} Debian package found: $SIZE"
else
    echo -e "${RED}✗${NC} Debian package not found"
    exit 1
fi

if [ -f "target/release/bundle/appimage/meetily_0.2.0_amd64.AppImage" ]; then
    SIZE=$(du -h target/release/bundle/appimage/meetily_0.2.0_amd64.AppImage | cut -f1)
    echo -e "${GREEN}✓${NC} AppImage found: $SIZE"
else
    echo -e "${RED}✗${NC} AppImage not found"
    exit 1
fi

echo ""

# Check if llama-helper binary exists
echo "🦙 Checking llama-helper sidecar..."
if [ -f "frontend/src-tauri/binaries/llama-helper-x86_64-unknown-linux-gnu" ]; then
    echo -e "${GREEN}✓${NC} llama-helper binary found"
else
    echo -e "${YELLOW}⚠${NC} llama-helper binary not found (may be bundled)"
fi

echo ""

# Check Vulkan support
echo "🎮 Checking Vulkan GPU support..."
if command -v vulkaninfo &> /dev/null; then
    if vulkaninfo --summary &> /dev/null; then
        echo -e "${GREEN}✓${NC} Vulkan is available"
        GPU=$(vulkaninfo | grep "deviceName" | head -1 | cut -d'=' -f2 | xargs)
        echo "  GPU: $GPU"
    else
        echo -e "${YELLOW}⚠${NC} Vulkan installed but no compatible GPU found"
    fi
else
    echo -e "${YELLOW}⚠${NC} vulkaninfo not found (install vulkan-tools to check)"
fi

echo ""

# Check Python environment
echo "🐍 Checking Python environment for diarization..."
if [ -d "frontend/src-tauri/python/venv" ]; then
    echo -e "${GREEN}✓${NC} Python virtual environment exists"
    
    # Check if pyannote is installed
    if [ -f "frontend/src-tauri/python/venv/bin/python" ]; then
        if frontend/src-tauri/python/venv/bin/python -c "import pyannote.audio" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} pyannote.audio is installed"
        else
            echo -e "${YELLOW}⚠${NC} pyannote.audio not installed"
            echo "  Run: cd frontend/src-tauri/python && ./setup_diarization.sh"
        fi
    fi
else
    echo -e "${YELLOW}⚠${NC} Python environment not set up"
    echo "  Run: cd frontend/src-tauri/python && ./setup_diarization.sh"
fi

echo ""

# Check database migration
echo "💾 Checking database schema..."
if [ -f "frontend/src-tauri/migrations/20260209000000_add_speaker_diarization.sql" ]; then
    echo -e "${GREEN}✓${NC} Speaker diarization migration exists"
else
    echo -e "${RED}✗${NC} Migration file not found"
fi

echo ""

# Check UI components
echo "🎨 Checking UI components..."
COMPONENTS=(
    "frontend/src/components/SpeakerLabel.tsx"
    "frontend/src/components/SpeakerStatisticsView.tsx"
    "frontend/src/components/DiarizationSettings.tsx"
)

for component in "${COMPONENTS[@]}"; do
    if [ -f "$component" ]; then
        echo -e "${GREEN}✓${NC} $(basename $component)"
    else
        echo -e "${RED}✗${NC} $(basename $component) not found"
    fi
done

echo ""

# Check documentation
echo "📚 Checking documentation..."
DOCS=(
    "frontend/src-tauri/src/diarization/USER_GUIDE.md"
    "frontend/src-tauri/src/diarization/DEVELOPER_GUIDE.md"
    "frontend/src-tauri/src/diarization/UI_INTEGRATION_GUIDE.md"
    "SPEAKER_DIARIZATION_DEPLOYMENT.md"
)

for doc in "${DOCS[@]}"; do
    if [ -f "$doc" ]; then
        echo -e "${GREEN}✓${NC} $(basename $doc)"
    else
        echo -e "${YELLOW}⚠${NC} $(basename $doc) not found"
    fi
done

echo ""
echo "=================================================="
echo -e "${GREEN}✅ Build verification complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Install: sudo dpkg -i target/release/bundle/deb/meetily_0.2.0_amd64.deb"
echo "2. Setup Python: cd frontend/src-tauri/python && ./setup_diarization.sh"
echo "3. Launch: meetily"
echo "4. Enable diarization: Settings → Speakers → Enable Speaker Diarization"
echo ""
echo "For more information, see SPEAKER_DIARIZATION_DEPLOYMENT.md"
