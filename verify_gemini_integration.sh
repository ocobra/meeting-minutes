#!/bin/bash

echo "========================================="
echo "Meetily Gemini Integration Verification"
echo "========================================="
echo ""

# Check if GEMINI_API_KEY is set
echo "1. Checking GEMINI_API_KEY environment variable..."
if [ -z "$GEMINI_API_KEY" ]; then
    echo "   ❌ GEMINI_API_KEY is NOT set"
    echo "   Please run: export GEMINI_API_KEY='your_key_here'"
    exit 1
else
    echo "   ✅ GEMINI_API_KEY is set: ${GEMINI_API_KEY:0:20}..."
fi
echo ""

# Check if Meetily binary exists
echo "2. Checking Meetily installation..."
if [ -f "/usr/bin/meetily" ]; then
    echo "   ✅ Meetily is installed at /usr/bin/meetily"
    echo "   Size: $(ls -lh /usr/bin/meetily | awk '{print $5}')"
    echo "   Modified: $(ls -l /usr/bin/meetily | awk '{print $6, $7, $8}')"
else
    echo "   ❌ Meetily is NOT installed"
    exit 1
fi
echo ""

# Check if Gemini support is compiled in
echo "3. Checking Gemini support in binary..."
if strings /usr/bin/meetily | grep -q "Gemini"; then
    echo "   ✅ Gemini support found in binary"
    echo "   References found:"
    strings /usr/bin/meetily | grep -i "gemini" | head -3 | sed 's/^/      - /'
else
    echo "   ❌ Gemini support NOT found in binary"
    exit 1
fi
echo ""

# Check if Vulkan is enabled
echo "4. Checking Vulkan GPU acceleration..."
if ldd /usr/bin/meetily | grep -q "libvulkan"; then
    echo "   ✅ Vulkan is enabled"
    ldd /usr/bin/meetily | grep vulkan | sed 's/^/      /'
else
    echo "   ⚠️  Vulkan NOT found (GPU acceleration may not work)"
fi
echo ""

# Check if HUGGINGFACE_API_KEY is set
echo "5. Checking HUGGINGFACE_API_KEY (for speaker segmentation)..."
if [ -z "$HUGGINGFACE_API_KEY" ]; then
    echo "   ⚠️  HUGGINGFACE_API_KEY is NOT set"
    echo "   Speaker segmentation will use local pyannote.audio"
else
    echo "   ✅ HUGGINGFACE_API_KEY is set: ${HUGGINGFACE_API_KEY:0:20}..."
fi
echo ""

# Check Python environment for diarization
echo "6. Checking Python environment for speaker diarization..."
if [ -d "frontend/src-tauri/python/venv" ]; then
    echo "   ✅ Python virtual environment exists"
    if [ -f "frontend/src-tauri/python/venv/bin/python" ]; then
        PYTHON_VERSION=$(frontend/src-tauri/python/venv/bin/python --version 2>&1)
        echo "   Python version: $PYTHON_VERSION"
    fi
else
    echo "   ⚠️  Python virtual environment NOT found"
    echo "   Run: cd frontend/src-tauri/python && ./setup_diarization.sh"
fi
echo ""

# Summary
echo "========================================="
echo "Summary"
echo "========================================="
echo ""
echo "✅ Meetily v0.2.0 with Gemini support is installed"
echo "✅ GEMINI_API_KEY is configured"
echo ""
echo "Gemini will be used for:"
echo "  - Meeting summaries (already configured)"
echo "  - Speaker name identification (NEW!)"
echo ""
echo "To test:"
echo "  1. Launch Meetily: meetily"
echo "  2. Go to Settings → Speakers"
echo "  3. Enable speaker diarization"
echo "  4. Record a meeting with introductions"
echo "  5. Check logs: tail -f ~/.local/share/meetily/logs/*.log | grep -i gemini"
echo ""
echo "Expected log output:"
echo '  INFO Using Google Gemini API for speaker identification'
echo ""
echo "========================================="
