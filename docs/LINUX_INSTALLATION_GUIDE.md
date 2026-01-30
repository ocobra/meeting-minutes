# 🐧 Complete Linux Installation Guide for Meetily

This comprehensive guide will help you install and configure Meetily on Linux with full Ollama integration for local AI processing.

## 📋 Table of Contents

- [System Requirements](#-system-requirements)
- [Quick Installation](#-quick-installation)
- [Detailed Installation Steps](#-detailed-installation-steps)
- [Ollama Setup and Integration](#-ollama-setup-and-integration)
- [GPU Acceleration Setup](#-gpu-acceleration-setup)
- [Build and Development](#-build-and-development)
- [Troubleshooting](#-troubleshooting)
- [Configuration Examples](#-configuration-examples)

---

## 🎯 System Requirements

### Minimum Requirements
- **OS**: Ubuntu 20.04+, Debian 11+, Fedora 35+, Arch Linux, or compatible
- **CPU**: 4 cores (Intel/AMD x86_64)
- **RAM**: 8GB (16GB recommended for large models)
- **Storage**: 20GB free space (more for additional models)
- **Network**: Internet connection for initial setup

### Recommended Requirements
- **CPU**: 8+ cores with AVX2 support
- **RAM**: 16GB+ (32GB for large language models)
- **GPU**: NVIDIA RTX series, AMD RX 6000+, or Intel Arc
- **Storage**: 50GB+ SSD space

### Supported Distributions

| Distribution | Version | Status | Notes |
|--------------|---------|--------|-------|
| Ubuntu | 20.04+ | ✅ Fully Supported | Recommended |
| Debian | 11+ | ✅ Fully Supported | Stable |
| Fedora | 35+ | ✅ Fully Supported | Latest packages |
| Arch Linux | Rolling | ✅ Fully Supported | Cutting edge |
| CentOS/RHEL | 8+ | ⚠️ Limited | Manual compilation |
| openSUSE | 15.4+ | ⚠️ Limited | Community tested |

---

## 🚀 Quick Installation

For experienced users who want to get started immediately:

```bash
# 1. Install system dependencies
sudo apt update && sudo apt install -y build-essential cmake git curl nodejs npm python3 python3-pip python3-venv

# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 3. Install pnpm (recommended)
npm install -g pnpm

# 4. Clone and build Meetily
git clone https://github.com/ocobra/meeting-minutes.git
cd meeting-minutes

# 5. Build with GPU auto-detection
cd frontend && ./build-gpu.sh

# 6. Install and configure Ollama
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull llama3.2:3b

# 7. Start services
cd ../backend && ./clean_start_backend.sh
```

---

## 📦 Detailed Installation Steps

### Step 1: System Dependencies

#### Ubuntu/Debian
```bash
# Update package lists
sudo apt update && sudo apt upgrade -y

# Essential build tools
sudo apt install -y \
    build-essential \
    cmake \
    git \
    curl \
    wget \
    pkg-config \
    libssl-dev \
    libsqlite3-dev

# Audio system dependencies
sudo apt install -y \
    libasound2-dev \
    libpulse-dev \
    libjack-jackd2-dev \
    portaudio19-dev

# Python development
sudo apt install -y \
    python3 \
    python3-pip \
    python3-venv \
    python3-dev

# Node.js (latest LTS)
curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
sudo apt install -y nodejs
```

#### Fedora/RHEL
```bash
# Update system
sudo dnf update -y

# Development tools
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y \
    cmake \
    git \
    curl \
    wget \
    pkg-config \
    openssl-devel \
    sqlite-devel

# Audio system
sudo dnf install -y \
    alsa-lib-devel \
    pulseaudio-libs-devel \
    jack-audio-connection-kit-devel \
    portaudio-devel

# Python
sudo dnf install -y \
    python3 \
    python3-pip \
    python3-devel

# Node.js
sudo dnf install -y nodejs npm
```

#### Arch Linux
```bash
# Update system
sudo pacman -Syu

# Development tools
sudo pacman -S --needed \
    base-devel \
    cmake \
    git \
    curl \
    wget \
    pkg-config \
    openssl \
    sqlite

# Audio system
sudo pacman -S --needed \
    alsa-lib \
    libpulse \
    jack2 \
    portaudio

# Python and Node.js
sudo pacman -S --needed \
    python \
    python-pip \
    nodejs \
    npm
```

### Step 2: Rust Installation

```bash
# Install Rust using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow the prompts (choose default installation)
# Reload environment
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### Step 3: Package Manager Setup

```bash
# Install pnpm (recommended for faster builds)
npm install -g pnpm

# Verify installation
pnpm --version

# Alternative: Use npm (slower but works)
# npm --version
```

### Step 4: Clone Meetily Repository

```bash
# Clone your fork
git clone https://github.com/ocobra/meeting-minutes.git
cd meeting-minutes

# Check repository structure
ls -la
```

---

## 🦙 Ollama Setup and Integration

### Installing Ollama

#### Method 1: Official Installer (Recommended)
```bash
# Download and install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Verify installation
ollama --version
```

#### Method 2: Manual Installation
```bash
# Download binary
curl -L https://ollama.ai/download/ollama-linux-amd64 -o ollama
chmod +x ollama
sudo mv ollama /usr/local/bin/

# Create systemd service
sudo tee /etc/systemd/system/ollama.service > /dev/null <<EOF
[Unit]
Description=Ollama Service
After=network-online.target

[Service]
ExecStart=/usr/local/bin/ollama serve
User=ollama
Group=ollama
Restart=always
RestartSec=3
Environment="PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
Environment="OLLAMA_HOST=0.0.0.0"

[Install]
WantedBy=default.target
EOF

# Create ollama user
sudo useradd -r -s /bin/false -m -d /usr/share/ollama ollama

# Start service
sudo systemctl daemon-reload
sudo systemctl enable ollama
sudo systemctl start ollama
```

### Configuring Ollama for Meetily

#### 1. Download Recommended Models
```bash
# Start Ollama service
sudo systemctl start ollama

# Download models (choose based on your system)
# For systems with 8GB+ RAM:
ollama pull llama3.2:3b

# For systems with 16GB+ RAM:
ollama pull llama3.2:7b

# For systems with 32GB+ RAM:
ollama pull llama3.2:70b

# Lightweight alternative:
ollama pull phi3:mini

# Verify models
ollama list
```

#### 2. Test Ollama Integration
```bash
# Test basic functionality
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2:3b",
  "prompt": "Hello, how are you?",
  "stream": false
}'

# Expected response: JSON with generated text
```

#### 3. Configure Meetily Backend
```bash
# Navigate to backend directory
cd backend

# Create environment file
cat > .env << EOF
# Ollama Configuration
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b

# Database Configuration
DATABASE_PATH=./data/meeting_minutes.db

# Optional: API Keys for cloud providers (if needed)
# ANTHROPIC_API_KEY=your_claude_key_here
# GROQ_API_KEY=your_groq_key_here
# OPENAI_API_KEY=your_openai_key_here
EOF

# Set proper permissions
chmod 600 .env
```

---

## 🎮 GPU Acceleration Setup

### NVIDIA GPU (CUDA)

#### Prerequisites Check
```bash
# Check if NVIDIA GPU is present
lspci | grep -i nvidia

# Check NVIDIA driver
nvidia-smi

# Check compute capability (should be 5.0+)
nvidia-smi --query-gpu=compute_cap --format=csv
```

#### Install CUDA Toolkit
```bash
# Ubuntu/Debian
sudo apt install nvidia-cuda-toolkit nvidia-driver-550

# Fedora
sudo dnf install cuda-toolkit nvidia-driver

# Arch Linux
sudo pacman -S cuda nvidia nvidia-utils

# Verify installation
nvcc --version
```

#### Configure for Meetily
```bash
# Set environment variables
echo 'export CUDA_PATH=/usr/local/cuda' >> ~/.bashrc
echo 'export PATH=$CUDA_PATH/bin:$PATH' >> ~/.bashrc
echo 'export LD_LIBRARY_PATH=$CUDA_PATH/lib64:$LD_LIBRARY_PATH' >> ~/.bashrc
source ~/.bashrc

# Configure Ollama for GPU
sudo systemctl edit ollama
# Add:
# [Service]
# Environment="OLLAMA_GPU=1"

sudo systemctl restart ollama
```

### AMD GPU (ROCm)

#### Install ROCm
```bash
# Ubuntu/Debian (add ROCm repository first)
wget -q -O - https://repo.radeon.com/rocm/rocm.gpg.key | sudo apt-key add -
echo 'deb [arch=amd64] https://repo.radeon.com/rocm/apt/debian/ ubuntu main' | sudo tee /etc/apt/sources.list.d/rocm.list
sudo apt update
sudo apt install rocm-dev rocm-smi

# Fedora
sudo dnf install rocm-dev rocm-smi

# Set environment
echo 'export ROCM_PATH=/opt/rocm' >> ~/.bashrc
source ~/.bashrc

# Verify
rocm-smi
```

### Intel GPU (Vulkan)

#### Install Vulkan SDK
```bash
# Ubuntu/Debian
sudo apt install vulkan-sdk libopenblas-dev

# Fedora
sudo dnf install vulkan-devel openblas-devel

# Arch Linux
sudo pacman -S vulkan-devel openblas

# Configure environment
echo 'export VULKAN_SDK=/usr' >> ~/.bashrc
echo 'export BLAS_INCLUDE_DIRS=/usr/include/x86_64-linux-gnu' >> ~/.bashrc
source ~/.bashrc
```

---

## 🔨 Build and Development

### Building Meetily

#### 1. Frontend Build
```bash
cd frontend

# Install dependencies
pnpm install

# Build with automatic GPU detection
./build-gpu.sh

# Alternative: Force specific GPU feature
TAURI_GPU_FEATURE=cuda ./build-gpu.sh    # NVIDIA
TAURI_GPU_FEATURE=vulkan ./build-gpu.sh  # AMD/Intel
TAURI_GPU_FEATURE=hipblas ./build-gpu.sh # AMD ROCm
```

#### 2. Backend Setup
```bash
cd ../backend

# Create Python virtual environment
python3 -m venv venv
source venv/bin/activate

# Install Python dependencies
pip install -r requirements.txt

# Build Whisper server
./build_whisper.sh small

# Setup database
./setup-db.sh
```

### Development Mode

#### Start Development Environment
```bash
# Terminal 1: Start backend services
cd backend
./clean_start_backend.sh

# Terminal 2: Start frontend development
cd frontend
./dev-gpu.sh
```

#### Verify Installation
```bash
# Check services are running
curl http://localhost:8178/  # Whisper server
curl http://localhost:5167/get-meetings  # Backend API
curl http://localhost:11434/api/tags  # Ollama

# Check frontend (should open application)
# Frontend runs on http://localhost:3118 in development
```

---

## 🔧 Troubleshooting

### Common Issues and Solutions

#### 1. Build Failures

**Issue**: `cargo build` fails with linking errors
```bash
# Solution: Install missing development libraries
sudo apt install build-essential cmake pkg-config libssl-dev

# For audio issues:
sudo apt install libasound2-dev libpulse-dev
```

**Issue**: Node.js version conflicts
```bash
# Solution: Use Node Version Manager
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
source ~/.bashrc
nvm install --lts
nvm use --lts
```

#### 2. GPU Detection Issues

**Issue**: GPU not detected during build
```bash
# Check GPU detection manually
cd frontend
node scripts/auto-detect-gpu.js

# Force GPU feature if detection fails
TAURI_GPU_FEATURE=cuda ./build-gpu.sh
```

**Issue**: CUDA toolkit not found
```bash
# Verify CUDA installation
nvcc --version
which nvcc

# Set CUDA path manually
export CUDA_PATH=/usr/local/cuda
export PATH=$CUDA_PATH/bin:$PATH
```

#### 3. Ollama Issues

**Issue**: Ollama service not starting
```bash
# Check service status
sudo systemctl status ollama

# Check logs
sudo journalctl -u ollama -f

# Restart service
sudo systemctl restart ollama
```

**Issue**: Model download fails
```bash
# Check disk space
df -h

# Download manually with retry
ollama pull llama3.2:3b --retry 3

# Use smaller model if memory limited
ollama pull phi3:mini
```

#### 4. Audio System Issues

**Issue**: No audio devices detected
```bash
# Check audio system
aplay -l  # List playback devices
arecord -l  # List recording devices

# Install missing audio libraries
sudo apt install pulseaudio pulseaudio-utils

# Restart audio system
pulseaudio --kill
pulseaudio --start
```

#### 5. Permission Issues

**Issue**: Permission denied errors
```bash
# Fix audio group membership
sudo usermod -a -G audio $USER

# Fix microphone permissions
sudo chmod 666 /dev/snd/*

# Logout and login again
```

### Performance Optimization

#### 1. System Tuning
```bash
# Increase file descriptor limits
echo '* soft nofile 65536' | sudo tee -a /etc/security/limits.conf
echo '* hard nofile 65536' | sudo tee -a /etc/security/limits.conf

# Optimize for audio processing
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.conf
```

#### 2. Ollama Optimization
```bash
# Configure Ollama for better performance
sudo systemctl edit ollama
# Add:
# [Service]
# Environment="OLLAMA_NUM_PARALLEL=4"
# Environment="OLLAMA_MAX_LOADED_MODELS=2"
# Environment="OLLAMA_FLASH_ATTENTION=1"

sudo systemctl restart ollama
```

---

## ⚙️ Configuration Examples

### 1. Complete Environment Configuration

#### ~/.bashrc additions
```bash
# Meetily Development Environment
export MEETILY_HOME="$HOME/meeting-minutes"

# Rust environment
export PATH="$HOME/.cargo/bin:$PATH"

# CUDA (if using NVIDIA GPU)
export CUDA_PATH="/usr/local/cuda"
export PATH="$CUDA_PATH/bin:$PATH"
export LD_LIBRARY_PATH="$CUDA_PATH/lib64:$LD_LIBRARY_PATH"

# ROCm (if using AMD GPU)
export ROCM_PATH="/opt/rocm"
export PATH="$ROCM_PATH/bin:$PATH"

# Vulkan (if using Intel/other GPU)
export VULKAN_SDK="/usr"
export BLAS_INCLUDE_DIRS="/usr/include/x86_64-linux-gnu"

# Ollama
export OLLAMA_HOST="http://localhost:11434"

# Audio optimization
export PULSE_LATENCY_MSEC=30
```

### 2. Systemd Service for Meetily Backend

#### /etc/systemd/system/meetily-backend.service
```ini
[Unit]
Description=Meetily Backend Service
After=network-online.target ollama.service
Wants=ollama.service

[Service]
Type=simple
User=meetily
Group=meetily
WorkingDirectory=/opt/meetily/backend
Environment="PATH=/opt/meetily/venv/bin:/usr/local/bin:/usr/bin:/bin"
Environment="PYTHONPATH=/opt/meetily/backend"
Environment="DATABASE_PATH=/opt/meetily/data/meeting_minutes.db"
Environment="OLLAMA_HOST=http://localhost:11434"
ExecStart=/opt/meetily/venv/bin/python main.py
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
```

### 3. Nginx Reverse Proxy Configuration

#### /etc/nginx/sites-available/meetily
```nginx
server {
    listen 80;
    server_name meetily.local;

    # Frontend
    location / {
        proxy_pass http://localhost:3118;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }

    # Backend API
    location /api/ {
        proxy_pass http://localhost:5167/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Whisper Server
    location /whisper/ {
        proxy_pass http://localhost:8178/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 4. Docker Compose for Production

#### docker-compose.production.yml
```yaml
version: '3.8'

services:
  ollama:
    image: ollama/ollama:latest
    container_name: meetily-ollama
    restart: unless-stopped
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    environment:
      - OLLAMA_HOST=0.0.0.0
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: all
              capabilities: [gpu]

  meetily-backend:
    build: ./backend
    container_name: meetily-backend
    restart: unless-stopped
    ports:
      - "5167:5167"
    volumes:
      - ./data:/app/data
    environment:
      - OLLAMA_HOST=http://ollama:11434
      - DATABASE_PATH=/app/data/meeting_minutes.db
    depends_on:
      - ollama

  whisper-server:
    build: 
      context: ./backend
      dockerfile: Dockerfile.server-cpu
    container_name: meetily-whisper
    restart: unless-stopped
    ports:
      - "8178:8178"
    volumes:
      - whisper_models:/app/models

volumes:
  ollama_data:
  whisper_models:
```

---

## 🎯 Next Steps

After successful installation:

1. **Test the complete workflow**:
   ```bash
   # Start all services
   cd backend && ./clean_start_backend.sh
   
   # In another terminal, start frontend
   cd frontend && ./dev-gpu.sh
   ```

2. **Configure your first meeting**:
   - Open the application
   - Configure audio devices
   - Test recording and transcription
   - Generate your first AI summary

3. **Optimize for your hardware**:
   - Monitor GPU usage during transcription
   - Adjust Ollama model size based on available RAM
   - Fine-tune audio settings for your environment

4. **Set up automation** (optional):
   - Create systemd services for auto-start
   - Configure log rotation
   - Set up backup scripts for meeting data

---

## 📚 Additional Resources

- [Meetily Documentation](../README.md)
- [GPU Acceleration Guide](../docs/GPU_ACCELERATION.md)
- [Building from Source](../docs/BUILDING.md)
- [Ollama Documentation](https://ollama.ai/docs)
- [Tauri Documentation](https://tauri.app/v1/guides/)

---

## 🆘 Getting Help

If you encounter issues:

1. **Check the logs**:
   ```bash
   # Backend logs
   tail -f backend/logs/*.log
   
   # Ollama logs
   sudo journalctl -u ollama -f
   
   # System logs
   dmesg | tail
   ```

2. **Community Support**:
   - [GitHub Issues](https://github.com/ocobra/meeting-minutes/issues)
   - [Discord Community](https://discord.gg/crRymMQBFH)
   - [Reddit Community](https://www.reddit.com/r/meetily/)

3. **Provide system information**:
   ```bash
   # System info
   uname -a
   lsb_release -a
   
   # Hardware info
   lscpu
   lsmem
   lspci | grep -E "(VGA|3D)"
   
   # Software versions
   rustc --version
   node --version
   python3 --version
   ollama --version
   ```

---

**Installation complete!** 🎉 You now have a fully functional Meetily installation with local AI processing via Ollama.