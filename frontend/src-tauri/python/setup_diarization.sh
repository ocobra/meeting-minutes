#!/bin/bash
# Setup script for speaker diarization Python dependencies

set -e

echo "=== Speaker Diarization Setup ==="
echo ""

# Check Python version
echo "Checking Python version..."
python3 --version || {
    echo "Error: Python 3 not found. Please install Python 3.8 or later."
    exit 1
}

# Check if pip is available
echo "Checking pip..."
python3 -m pip --version || {
    echo "Error: pip not found. Please install pip."
    exit 1
}

# Create virtual environment (optional but recommended)
read -p "Create virtual environment? (recommended) [y/N]: " create_venv
if [[ $create_venv =~ ^[Yy]$ ]]; then
    echo "Creating virtual environment..."
    python3 -m venv venv
    source venv/bin/activate
    echo "Virtual environment activated"
fi

# Upgrade pip
echo "Upgrading pip..."
python3 -m pip install --upgrade pip

# Install PyTorch (with CUDA support if available)
echo ""
echo "Installing PyTorch..."
if command -v nvidia-smi &> /dev/null; then
    echo "NVIDIA GPU detected, installing PyTorch with CUDA support..."
    python3 -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118
else
    echo "No NVIDIA GPU detected, installing CPU-only PyTorch..."
    python3 -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
fi

# Install diarization dependencies
echo ""
echo "Installing diarization dependencies..."
python3 -m pip install -r requirements-diarization.txt

# Verify installation
echo ""
echo "Verifying installation..."
python3 -c "import torch; print(f'PyTorch version: {torch.__version__}')"
python3 -c "import pyannote.audio; print('pyannote.audio installed successfully')"

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo "1. Get a Hugging Face token from https://huggingface.co/settings/tokens"
echo "2. Accept the model license at https://huggingface.co/pyannote/speaker-diarization-3.1"
echo "3. Set environment variable: export HUGGINGFACE_API_KEY='your_token'"
echo ""
echo "Test the installation:"
echo "  python3 diarization_engine.py --audio_path /path/to/audio.wav --auth_token YOUR_TOKEN"
echo ""
