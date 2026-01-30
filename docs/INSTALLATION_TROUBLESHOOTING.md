# Meetily Installation Troubleshooting Guide

This comprehensive troubleshooting guide helps resolve common installation and configuration issues when setting up Meetily on Linux systems.

## Table of Contents

1. [System Requirements Issues](#system-requirements-issues)
2. [Dependency Installation Problems](#dependency-installation-problems)
3. [Rust and Cargo Issues](#rust-and-cargo-issues)
4. [Node.js and npm Problems](#nodejs-and-npm-problems)
5. [Python and Backend Issues](#python-and-backend-issues)
6. [Ollama Integration Problems](#ollama-integration-problems)
7. [GPU Acceleration Issues](#gpu-acceleration-issues)
8. [Audio System Problems](#audio-system-problems)
9. [Database and Storage Issues](#database-and-storage-issues)
10. [Network and CORS Problems](#network-and-cors-problems)
11. [Build and Compilation Errors](#build-and-compilation-errors)
12. [Runtime and Performance Issues](#runtime-and-performance-issues)
13. [Validation Scripts](#validation-scripts)

## System Requirements Issues

### Insufficient System Resources

**Problem**: Application fails to start or performs poorly due to insufficient system resources.

**Symptoms**:
- Out of memory errors during model loading
- Slow transcription processing
- Application crashes under load

**Solutions**:
```bash
# Check system resources
free -h
df -h
lscpu

# Minimum requirements check
if [ $(free -m | awk 'NR==2{print $2}') -lt 8192 ]; then
    echo "Warning: Less than 8GB RAM available"
fi

# Recommended: 16GB+ RAM for optimal performance
# Adjust swap if needed
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Unsupported Linux Distribution

**Problem**: Installation fails on unsupported or older Linux distributions.

**Symptoms**:
- Package manager cannot find required packages
- Compilation errors due to outdated system libraries
- Missing system dependencies

**Solutions**:
```bash
# Check distribution compatibility
lsb_release -a

# Supported distributions:
# - Ubuntu 20.04+ / Debian 11+
# - Fedora 35+ / CentOS Stream 9+
# - Arch Linux (rolling)
# - openSUSE Leap 15.4+

# For older distributions, consider using Docker:
docker --version || curl -fsSL https://get.docker.com -o get-docker.sh && sh get-docker.sh
```
## Dependency Installation Problems

### Package Manager Issues

**Problem**: System package installation fails or packages are not found.

**Symptoms**:
- `apt`, `yum`, or `pacman` commands fail
- Missing development headers
- Library version conflicts

**Solutions**:

**Ubuntu/Debian**:
```bash
# Update package lists
sudo apt update && sudo apt upgrade -y

# Install build essentials
sudo apt install -y build-essential pkg-config libssl-dev

# Fix broken packages
sudo apt --fix-broken install
sudo dpkg --configure -a

# Alternative repositories for newer packages
sudo add-apt-repository ppa:deadsnakes/ppa  # For Python
```

**Fedora/RHEL/CentOS**:
```bash
# Update system
sudo dnf update -y

# Install development tools
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y openssl-devel pkg-config

# Enable additional repositories
sudo dnf install -y epel-release
```

**Arch Linux**:
```bash
# Update system
sudo pacman -Syu

# Install base development packages
sudo pacman -S base-devel openssl pkg-config

# Install AUR helper if needed
git clone https://aur.archlinux.org/yay.git
cd yay && makepkg -si
```

### Missing System Libraries

**Problem**: Compilation fails due to missing system libraries.

**Common Missing Libraries**:
```bash
# Audio libraries
sudo apt install -y libasound2-dev libpulse-dev  # Ubuntu/Debian
sudo dnf install -y alsa-lib-devel pulseaudio-libs-devel  # Fedora
sudo pacman -S alsa-lib libpulse  # Arch

# SSL/TLS libraries
sudo apt install -y libssl-dev  # Ubuntu/Debian
sudo dnf install -y openssl-devel  # Fedora
sudo pacman -S openssl  # Arch

# Database libraries
sudo apt install -y libsqlite3-dev  # Ubuntu/Debian
sudo dnf install -y sqlite-devel  # Fedora
sudo pacman -S sqlite  # Arch
```

## Rust and Cargo Issues

### Rust Installation Problems

**Problem**: Rust installation fails or cargo commands don't work.

**Symptoms**:
- `rustc` or `cargo` command not found
- Permission denied errors
- Outdated Rust version

**Solutions**:
```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version

# Update Rust if needed
rustup update

# Fix PATH issues
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Set default toolchain
rustup default stable
```

### Cargo Build Failures

**Problem**: Cargo build fails with compilation errors.

**Common Issues and Solutions**:
```bash
# Clear cargo cache
cargo clean

# Update dependencies
cargo update

# Build with verbose output for debugging
cargo build --verbose

# Fix linker issues on some distributions
sudo apt install -y gcc-multilib  # Ubuntu/Debian
export RUSTFLAGS="-C link-arg=-fuse-ld=gold"  # Use gold linker

# Memory issues during compilation
export CARGO_BUILD_JOBS=1  # Reduce parallel jobs
```
## Node.js and npm Problems

### Node.js Version Issues

**Problem**: Wrong Node.js version or npm installation problems.

**Symptoms**:
- `node` or `npm` command not found
- Version compatibility errors
- Package installation failures

**Solutions**:
```bash
# Install Node.js using NodeSource repository (Ubuntu/Debian)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs

# Install using nvm (recommended for version management)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc
nvm install 18
nvm use 18

# Verify installation
node --version  # Should be v18.x or higher
npm --version

# Fix npm permissions
mkdir ~/.npm-global
npm config set prefix '~/.npm-global'
echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.bashrc
source ~/.bashrc
```

### Frontend Build Issues

**Problem**: Frontend build fails with dependency or compilation errors.

**Solutions**:
```bash
cd frontend

# Clear npm cache
npm cache clean --force

# Delete node_modules and reinstall
rm -rf node_modules package-lock.json
npm install

# Fix peer dependency issues
npm install --legacy-peer-deps

# Build with increased memory
export NODE_OPTIONS="--max-old-space-size=4096"
npm run build

# Alternative: Use yarn instead of npm
npm install -g yarn
yarn install
yarn build
```

## Python and Backend Issues

### Python Version Problems

**Problem**: Wrong Python version or missing Python dependencies.

**Symptoms**:
- `python3` command not found
- Module import errors
- Virtual environment issues

**Solutions**:
```bash
# Install Python 3.8+ (Ubuntu/Debian)
sudo apt install -y python3 python3-pip python3-venv python3-dev

# Install Python 3.8+ (Fedora)
sudo dnf install -y python3 python3-pip python3-venv python3-devel

# Create virtual environment
cd backend
python3 -m venv venv
source venv/bin/activate

# Upgrade pip
pip install --upgrade pip

# Install requirements
pip install -r requirements.txt

# Fix common dependency issues
pip install --upgrade setuptools wheel
```

### FastAPI Backend Issues

**Problem**: Backend server fails to start or API endpoints don't work.

**Common Issues**:
```bash
cd backend

# Check if all dependencies are installed
pip list | grep -E "(fastapi|uvicorn|sqlalchemy)"

# Start backend with debug logging
python -m uvicorn app.main:app --host 0.0.0.0 --port 8000 --reload --log-level debug

# Check database initialization
python -c "from app.db import init_db; init_db()"

# Test API endpoints
curl http://localhost:8000/health
curl http://localhost:8000/docs  # Swagger UI
```
## Ollama Integration Problems

### Ollama Installation Issues

**Problem**: Ollama installation or model download fails.

**Symptoms**:
- `ollama` command not found
- Model download timeouts
- GPU detection failures

**Solutions**:
```bash
# Install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Verify installation
ollama --version

# Start Ollama service
sudo systemctl start ollama
sudo systemctl enable ollama

# Check service status
sudo systemctl status ollama

# Manual start if service fails
ollama serve &

# Test Ollama connection
curl http://localhost:11434/api/version
```

### Model Download Problems

**Problem**: Model downloads fail or are extremely slow.

**Solutions**:
```bash
# Download models manually with retry
for i in {1..3}; do
    ollama pull llama2:7b && break
    echo "Retry $i failed, trying again..."
    sleep 5
done

# Check available disk space
df -h

# Use smaller models for testing
ollama pull phi:2.7b  # Smaller model for testing

# Check model list
ollama list

# Remove and re-download corrupted models
ollama rm llama2:7b
ollama pull llama2:7b
```

### Ollama API Connection Issues

**Problem**: Meetily cannot connect to Ollama API.

**Symptoms**:
- Connection refused errors
- Timeout errors
- API endpoint not responding

**Solutions**:
```bash
# Check if Ollama is running
ps aux | grep ollama
netstat -tlnp | grep 11434

# Test API connectivity
curl -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{"model": "llama2:7b", "prompt": "Hello", "stream": false}'

# Check firewall settings
sudo ufw status
sudo ufw allow 11434

# Configure Ollama for external access (if needed)
export OLLAMA_HOST=0.0.0.0:11434
ollama serve

# Update Meetily configuration
# Edit frontend/.env or backend configuration to point to correct Ollama endpoint
```

## GPU Acceleration Issues

### GPU Detection Problems

**Problem**: GPU is not detected or not utilized for acceleration.

**Symptoms**:
- Slow model inference
- CPU-only processing warnings
- GPU memory not being used

**Solutions**:

**NVIDIA GPU**:
```bash
# Check NVIDIA driver installation
nvidia-smi

# Install NVIDIA drivers if missing
sudo apt install -y nvidia-driver-470  # Ubuntu/Debian
sudo dnf install -y nvidia-driver  # Fedora

# Install CUDA toolkit
sudo apt install -y nvidia-cuda-toolkit  # Ubuntu/Debian

# Verify CUDA installation
nvcc --version

# Test GPU with Ollama
CUDA_VISIBLE_DEVICES=0 ollama run llama2:7b
```

**AMD GPU (ROCm)**:
```bash
# Install ROCm (Ubuntu 20.04+)
wget -q -O - https://repo.radeon.com/rocm/rocm.gpg.key | sudo apt-key add -
echo 'deb [arch=amd64] https://repo.radeon.com/rocm/apt/debian/ ubuntu main' | sudo tee /etc/apt/sources.list.d/rocm.list
sudo apt update
sudo apt install -y rocm-dkms

# Add user to render group
sudo usermod -a -G render $USER

# Reboot required
sudo reboot

# Verify ROCm installation
rocm-smi
```

**Intel GPU**:
```bash
# Install Intel GPU drivers (Ubuntu 22.04+)
sudo apt install -y intel-gpu-tools
sudo apt install -y intel-opencl-icd

# Verify Intel GPU
intel_gpu_top
```
### GPU Memory Issues

**Problem**: GPU runs out of memory or allocation fails.

**Solutions**:
```bash
# Check GPU memory usage
nvidia-smi  # NVIDIA
rocm-smi    # AMD

# Use smaller models
ollama pull phi:2.7b      # 2.7B parameters
ollama pull mistral:7b    # 7B parameters instead of 13B+

# Configure GPU memory limits in Ollama
export OLLAMA_GPU_MEMORY_FRACTION=0.8  # Use 80% of GPU memory

# Enable GPU memory growth (if supported)
export OLLAMA_GPU_MEMORY_GROWTH=true
```

## Audio System Problems

### Audio Device Detection Issues

**Problem**: Audio devices are not detected or accessible.

**Symptoms**:
- No audio input devices listed
- Permission denied errors
- Audio recording fails

**Solutions**:
```bash
# Check audio devices
arecord -l  # List capture devices
aplay -l    # List playback devices

# Install audio utilities
sudo apt install -y alsa-utils pulseaudio-utils  # Ubuntu/Debian
sudo dnf install -y alsa-utils pulseaudio-utils  # Fedora

# Add user to audio group
sudo usermod -a -G audio $USER

# Restart audio services
sudo systemctl restart pulseaudio
sudo systemctl restart alsa-state

# Test audio recording
arecord -f cd -t wav -d 5 test.wav
aplay test.wav
```

### PulseAudio Configuration Issues

**Problem**: PulseAudio configuration prevents audio capture.

**Solutions**:
```bash
# Restart PulseAudio
pulseaudio -k
pulseaudio --start

# Check PulseAudio status
pulseaudio --check -v

# List PulseAudio sources
pactl list sources short

# Set default source
pactl set-default-source alsa_input.pci-0000_00_1f.3.analog-stereo

# Increase buffer sizes for better performance
echo "default-sample-rate = 48000" >> ~/.pulse/daemon.conf
echo "default-fragments = 8" >> ~/.pulse/daemon.conf
echo "default-fragment-size-msec = 10" >> ~/.pulse/daemon.conf
```

## Database and Storage Issues

### SQLite Database Problems

**Problem**: Database initialization or access fails.

**Symptoms**:
- Database locked errors
- Permission denied accessing database
- Corrupted database files

**Solutions**:
```bash
# Check database file permissions
ls -la ~/.local/share/meetily/

# Create database directory if missing
mkdir -p ~/.local/share/meetily/

# Fix database permissions
chmod 755 ~/.local/share/meetily/
chmod 644 ~/.local/share/meetily/meetily.db

# Test database connectivity
sqlite3 ~/.local/share/meetily/meetily.db ".tables"

# Backup and recreate corrupted database
cp ~/.local/share/meetily/meetily.db ~/.local/share/meetily/meetily.db.backup
rm ~/.local/share/meetily/meetily.db
# Restart application to recreate database
```

### Storage Space Issues

**Problem**: Insufficient disk space for recordings and models.

**Solutions**:
```bash
# Check disk usage
df -h
du -sh ~/.local/share/meetily/
du -sh ~/.ollama/

# Clean up old recordings (be careful!)
find ~/.local/share/meetily/recordings -name "*.wav" -mtime +30 -delete

# Move Ollama models to different location with more space
export OLLAMA_MODELS=/path/to/larger/storage
mkdir -p $OLLAMA_MODELS
mv ~/.ollama/models/* $OLLAMA_MODELS/

# Configure application storage location
# Edit configuration to use different storage path
```
## Network and CORS Problems

### CORS Configuration Issues

**Problem**: Frontend cannot connect to backend due to CORS errors.

**Symptoms**:
- Browser console shows CORS errors
- API requests fail from frontend
- Network tab shows preflight failures

**Solutions**:
```bash
# Check backend CORS configuration
grep -r "CORS" backend/app/

# Update CORS settings in backend/app/main.py
# Allow specific origins instead of "*"
```

Example CORS configuration:
```python
from fastapi.middleware.cors import CORSMiddleware

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000", "http://127.0.0.1:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

### Port Conflicts

**Problem**: Required ports are already in use.

**Solutions**:
```bash
# Check port usage
sudo netstat -tlnp | grep -E "(3000|8000|11434)"
sudo lsof -i :3000
sudo lsof -i :8000
sudo lsof -i :11434

# Kill processes using required ports
sudo kill -9 $(lsof -t -i:3000)
sudo kill -9 $(lsof -t -i:8000)

# Use alternative ports
# Frontend: npm run dev -- --port 3001
# Backend: uvicorn app.main:app --port 8001
# Ollama: OLLAMA_HOST=0.0.0.0:11435 ollama serve
```

## Build and Compilation Errors

### Tauri Build Issues

**Problem**: Tauri application build fails.

**Common Issues**:
```bash
cd frontend

# Install Tauri CLI
cargo install tauri-cli

# Install system dependencies for Tauri
sudo apt install -y libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev  # Ubuntu/Debian
sudo dnf install -y webkit2gtk3-devel gtk3-devel libappindicator-gtk3-devel  # Fedora

# Build Tauri app
cargo tauri build

# Debug build issues
cargo tauri build --verbose

# Clean build cache
cargo clean
rm -rf src-tauri/target
```

### Whisper.cpp Compilation Issues

**Problem**: Whisper.cpp fails to compile or link.

**Solutions**:
```bash
cd backend/whisper.cpp

# Install required build tools
sudo apt install -y cmake make g++  # Ubuntu/Debian
sudo dnf install -y cmake make gcc-c++  # Fedora

# Clean and rebuild
make clean
make -j$(nproc)

# Build with specific optimizations
make WHISPER_OPENBLAS=1  # Use OpenBLAS
make WHISPER_CUBLAS=1    # Use CUDA (if available)

# Fix linking issues
export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
sudo ldconfig
```

## Runtime and Performance Issues

### High Memory Usage

**Problem**: Application consumes excessive memory.

**Solutions**:
```bash
# Monitor memory usage
htop
ps aux --sort=-%mem | head

# Reduce model sizes
ollama pull phi:2.7b  # Use smaller models

# Configure memory limits
export OLLAMA_MAX_LOADED_MODELS=1
export OLLAMA_GPU_MEMORY_FRACTION=0.7

# Increase swap space
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Slow Performance

**Problem**: Application runs slowly or becomes unresponsive.

**Solutions**:
```bash
# Check CPU usage
top
htop

# Enable GPU acceleration
# Ensure GPU drivers are installed and configured

# Optimize audio processing
# Reduce audio quality in settings if needed

# Check for background processes
ps aux | grep -E "(ollama|whisper|meetily)"

# Increase process priority
sudo renice -10 $(pgrep ollama)
sudo renice -10 $(pgrep meetily)
```
## Validation Scripts

### System Validation Script

Create a comprehensive validation script to check system readiness:

```bash
#!/bin/bash
# save as: validate_system.sh

echo "=== Meetily System Validation ==="

# Check system requirements
echo "Checking system requirements..."
RAM_GB=$(free -g | awk 'NR==2{print $2}')
if [ $RAM_GB -lt 8 ]; then
    echo "❌ Insufficient RAM: ${RAM_GB}GB (minimum 8GB required)"
else
    echo "✅ RAM: ${RAM_GB}GB"
fi

DISK_GB=$(df -BG . | awk 'NR==2{print $4}' | sed 's/G//')
if [ $DISK_GB -lt 20 ]; then
    echo "❌ Insufficient disk space: ${DISK_GB}GB (minimum 20GB required)"
else
    echo "✅ Disk space: ${DISK_GB}GB"
fi

# Check required commands
echo "Checking required commands..."
for cmd in rustc cargo node npm python3 pip3; do
    if command -v $cmd >/dev/null 2>&1; then
        echo "✅ $cmd: $(command -v $cmd)"
    else
        echo "❌ $cmd: not found"
    fi
done

# Check Ollama
echo "Checking Ollama..."
if command -v ollama >/dev/null 2>&1; then
    echo "✅ Ollama installed: $(ollama --version)"
    if curl -s http://localhost:11434/api/version >/dev/null 2>&1; then
        echo "✅ Ollama service running"
    else
        echo "❌ Ollama service not running"
    fi
else
    echo "❌ Ollama not installed"
fi

# Check GPU
echo "Checking GPU acceleration..."
if command -v nvidia-smi >/dev/null 2>&1; then
    echo "✅ NVIDIA GPU detected"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
elif command -v rocm-smi >/dev/null 2>&1; then
    echo "✅ AMD GPU detected"
    rocm-smi --showproductname
else
    echo "⚠️  No GPU acceleration detected (CPU-only mode)"
fi

# Check audio system
echo "Checking audio system..."
if command -v arecord >/dev/null 2>&1; then
    AUDIO_DEVICES=$(arecord -l 2>/dev/null | grep -c "card")
    if [ $AUDIO_DEVICES -gt 0 ]; then
        echo "✅ Audio devices: $AUDIO_DEVICES found"
    else
        echo "❌ No audio devices found"
    fi
else
    echo "❌ Audio utilities not installed"
fi

echo "=== Validation Complete ==="
```

### Installation Test Script

Create a script to test the complete installation:

```bash
#!/bin/bash
# save as: test_installation.sh

echo "=== Testing Meetily Installation ==="

# Test backend
echo "Testing backend..."
cd backend
if [ -f "venv/bin/activate" ]; then
    source venv/bin/activate
    python -c "import app.main; print('✅ Backend imports successful')" || echo "❌ Backend import failed"
    
    # Start backend in background for testing
    python -m uvicorn app.main:app --host 127.0.0.1 --port 8000 &
    BACKEND_PID=$!
    sleep 5
    
    # Test API endpoint
    if curl -s http://127.0.0.1:8000/health >/dev/null 2>&1; then
        echo "✅ Backend API responding"
    else
        echo "❌ Backend API not responding"
    fi
    
    # Cleanup
    kill $BACKEND_PID 2>/dev/null
else
    echo "❌ Backend virtual environment not found"
fi

# Test frontend
echo "Testing frontend..."
cd ../frontend
if [ -f "package.json" ]; then
    if npm list >/dev/null 2>&1; then
        echo "✅ Frontend dependencies installed"
    else
        echo "❌ Frontend dependencies missing"
    fi
    
    # Test build
    if npm run build >/dev/null 2>&1; then
        echo "✅ Frontend build successful"
    else
        echo "❌ Frontend build failed"
    fi
else
    echo "❌ Frontend package.json not found"
fi

# Test Ollama integration
echo "Testing Ollama integration..."
if curl -s -X POST http://localhost:11434/api/generate \
   -H "Content-Type: application/json" \
   -d '{"model": "llama2:7b", "prompt": "test", "stream": false}' >/dev/null 2>&1; then
    echo "✅ Ollama API integration working"
else
    echo "❌ Ollama API integration failed"
fi

echo "=== Installation Test Complete ==="
```

Make the scripts executable:
```bash
chmod +x validate_system.sh test_installation.sh
```

## Getting Help

If you continue to experience issues after following this troubleshooting guide:

1. **Check the logs**: Look for error messages in application logs
2. **Search existing issues**: Check the GitHub repository for similar problems
3. **Create a detailed issue**: Include system information, error messages, and steps to reproduce
4. **Join the community**: Participate in discussions and ask for help

### Collecting Debug Information

When reporting issues, include this information:

```bash
# System information
uname -a
lsb_release -a
free -h
df -h

# Software versions
rustc --version
cargo --version
node --version
npm --version
python3 --version
ollama --version

# GPU information
nvidia-smi 2>/dev/null || echo "No NVIDIA GPU"
rocm-smi 2>/dev/null || echo "No AMD GPU"

# Audio information
arecord -l
pactl list sources short

# Process information
ps aux | grep -E "(ollama|meetily|whisper)"
netstat -tlnp | grep -E "(3000|8000|11434)"
```

This troubleshooting guide covers the most common installation and configuration issues. Keep it updated as new issues are discovered and resolved.