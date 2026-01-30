# 🦙 Ollama Integration Guide for Meetily

This guide provides comprehensive instructions for integrating Ollama with Meetily to enable local AI-powered meeting summarization without sending data to external services.

## 📋 Table of Contents

- [Overview](#-overview)
- [Installation Methods](#-installation-methods)
- [Model Selection and Management](#-model-selection-and-management)
- [Configuration](#-configuration)
- [Performance Optimization](#-performance-optimization)
- [GPU Acceleration](#-gpu-acceleration)
- [Advanced Configuration](#-advanced-configuration)
- [Troubleshooting](#-troubleshooting)
- [Best Practices](#-best-practices)

---

## 🎯 Overview

### What is Ollama?

Ollama is a lightweight, extensible framework for building and running large language models locally. It provides:

- **Local Processing**: No data leaves your machine
- **Multiple Models**: Support for Llama, Mistral, CodeLlama, and more
- **GPU Acceleration**: Automatic GPU detection and utilization
- **Simple API**: REST API compatible with OpenAI format
- **Model Management**: Easy model downloading and switching

### Why Use Ollama with Meetily?

- **Privacy**: Complete data sovereignty - transcripts never leave your system
- **Cost**: No API fees for AI processing
- **Reliability**: Works offline, no internet dependency for AI features
- **Customization**: Choose models optimized for your hardware and use case
- **Performance**: Local processing can be faster than cloud APIs

---

## 🚀 Installation Methods

### Method 1: Official Installer (Recommended)

#### Linux/macOS
```bash
# Download and install Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Verify installation
ollama --version
```

#### Windows
```powershell
# Download from https://ollama.ai/download/windows
# Run the installer and follow prompts

# Verify installation (in new terminal)
ollama --version
```

### Method 2: Package Managers

#### macOS (Homebrew)
```bash
brew install ollama
```

#### Linux (Snap)
```bash
sudo snap install ollama
```

#### Arch Linux (AUR)
```bash
yay -S ollama
```

### Method 3: Docker Installation

```bash
# Pull Ollama Docker image
docker pull ollama/ollama

# Run Ollama in container
docker run -d \
  --name ollama \
  -p 11434:11434 \
  -v ollama:/root/.ollama \
  ollama/ollama

# For GPU support (NVIDIA)
docker run -d \
  --name ollama \
  --gpus all \
  -p 11434:11434 \
  -v ollama:/root/.ollama \
  ollama/ollama
```

### Method 4: Manual Installation

#### Linux
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

---

## 🧠 Model Selection and Management

### Recommended Models for Meetily

#### For Systems with 8GB RAM
| Model | Size | Use Case | Performance |
|-------|------|----------|-------------|
| `phi3:mini` | 2.3GB | Basic summaries | Fast, efficient |
| `llama3.2:3b` | 2.0GB | Good quality summaries | Balanced |
| `mistral:7b-instruct` | 4.1GB | High-quality summaries | Better quality |

#### For Systems with 16GB+ RAM
| Model | Size | Use Case | Performance |
|-------|------|----------|-------------|
| `llama3.2:7b` | 4.7GB | Excellent summaries | High quality |
| `llama3.1:8b` | 4.7GB | Professional summaries | Very good |
| `mistral:7b` | 4.1GB | Technical content | Specialized |

#### For Systems with 32GB+ RAM
| Model | Size | Use Case | Performance |
|-------|------|----------|-------------|
| `llama3.1:70b` | 40GB | Enterprise-grade | Exceptional |
| `codellama:34b` | 19GB | Technical meetings | Code-focused |

### Installing Models

#### Basic Installation
```bash
# Start Ollama service (if not running)
ollama serve &

# Download recommended models
ollama pull phi3:mini          # Lightweight (2.3GB)
ollama pull llama3.2:3b        # Balanced (2.0GB)
ollama pull llama3.2:7b        # High quality (4.7GB)

# List installed models
ollama list
```

#### Advanced Model Management
```bash
# Pull specific model versions
ollama pull llama3.2:3b-instruct-q4_0    # Quantized version
ollama pull llama3.2:3b-instruct-fp16     # Full precision

# Remove models to save space
ollama rm old-model-name

# Show model information
ollama show llama3.2:3b

# Copy/rename models
ollama cp llama3.2:3b my-custom-model
```

### Model Quantization Options

| Quantization | Size Reduction | Quality | Speed | Use Case |
|--------------|----------------|---------|-------|----------|
| `fp16` | Baseline | Highest | Slower | Best quality |
| `q8_0` | ~50% | Very High | Fast | Balanced |
| `q4_0` | ~75% | Good | Very Fast | Resource constrained |
| `q2_K` | ~85% | Lower | Fastest | Minimal resources |

```bash
# Example: Download different quantizations
ollama pull llama3.2:7b          # Default (usually q4_0)
ollama pull llama3.2:7b-q8_0     # Higher quality
ollama pull llama3.2:7b-q2_k     # Smaller size
```

---

## ⚙️ Configuration

### Basic Ollama Configuration

#### 1. Service Configuration
```bash
# Check if Ollama is running
curl http://localhost:11434/api/tags

# Start Ollama service
ollama serve

# Run in background (Linux/macOS)
nohup ollama serve > ollama.log 2>&1 &
```

#### 2. Environment Variables
```bash
# Create Ollama configuration
export OLLAMA_HOST="0.0.0.0:11434"        # Listen on all interfaces
export OLLAMA_ORIGINS="*"                  # Allow all origins (development)
export OLLAMA_NUM_PARALLEL=4               # Parallel requests
export OLLAMA_MAX_LOADED_MODELS=2          # Memory management
export OLLAMA_FLASH_ATTENTION=1            # Enable flash attention
export OLLAMA_GPU_OVERHEAD=0.1             # GPU memory overhead
```

#### 3. Persistent Configuration (Linux)
```bash
# Create configuration directory
sudo mkdir -p /etc/ollama

# Create configuration file
sudo tee /etc/ollama/ollama.conf > /dev/null <<EOF
OLLAMA_HOST=0.0.0.0:11434
OLLAMA_NUM_PARALLEL=4
OLLAMA_MAX_LOADED_MODELS=2
OLLAMA_FLASH_ATTENTION=1
EOF

# Update systemd service to use configuration
sudo systemctl edit ollama
# Add:
# [Service]
# EnvironmentFile=/etc/ollama/ollama.conf

sudo systemctl restart ollama
```

### Meetily Backend Configuration

#### 1. Environment Configuration
```bash
# Navigate to Meetily backend directory
cd backend

# Create or update .env file
cat > .env << EOF
# Ollama Configuration
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b
OLLAMA_TIMEOUT=300
OLLAMA_TEMPERATURE=0.7
OLLAMA_MAX_TOKENS=2048

# Fallback Configuration (optional)
ANTHROPIC_API_KEY=your_claude_key_here
GROQ_API_KEY=your_groq_key_here
OPENAI_API_KEY=your_openai_key_here

# Database Configuration
DATABASE_PATH=./data/meeting_minutes.db

# Logging
LOG_LEVEL=INFO
EOF

# Set proper permissions
chmod 600 .env
```

#### 2. Model Configuration in Meetily
```bash
# Test Ollama connection
curl -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3.2:3b",
    "prompt": "Summarize this meeting: We discussed project timeline and budget.",
    "stream": false
  }'
```

#### 3. Advanced Backend Configuration
```python
# backend/app/ollama_config.py
OLLAMA_CONFIG = {
    "host": "http://localhost:11434",
    "models": {
        "default": "llama3.2:3b",
        "fast": "phi3:mini",
        "quality": "llama3.2:7b",
        "technical": "codellama:7b"
    },
    "generation_params": {
        "temperature": 0.7,
        "top_p": 0.9,
        "top_k": 40,
        "repeat_penalty": 1.1,
        "max_tokens": 2048,
        "stop": ["Human:", "Assistant:"]
    },
    "timeout": 300,
    "retry_attempts": 3,
    "retry_delay": 5
}
```

---

## 🚀 Performance Optimization

### System-Level Optimizations

#### 1. Memory Management
```bash
# Increase swap if needed (for large models)
sudo fallocate -l 8G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# Make permanent
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Optimize swappiness for AI workloads
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.conf
```

#### 2. CPU Optimization
```bash
# Set CPU governor to performance
echo 'performance' | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Enable all CPU cores for Ollama
export OMP_NUM_THREADS=$(nproc)
export OLLAMA_NUM_PARALLEL=$(nproc)
```

#### 3. File System Optimization
```bash
# Use faster file system for model storage
# Mount with noatime for better performance
sudo mount -o remount,noatime /

# Create dedicated partition for models (optional)
sudo mkdir -p /opt/ollama/models
sudo chown ollama:ollama /opt/ollama/models
```

### Ollama-Specific Optimizations

#### 1. Model Loading Optimization
```bash
# Pre-load models to avoid cold start delays
ollama run llama3.2:3b "Hello" > /dev/null

# Keep models warm with periodic requests
while true; do
  curl -s -X POST http://localhost:11434/api/generate \
    -H "Content-Type: application/json" \
    -d '{"model": "llama3.2:3b", "prompt": "ping", "stream": false}' > /dev/null
  sleep 300  # Every 5 minutes
done &
```

#### 2. Concurrent Processing
```bash
# Configure for multiple simultaneous requests
export OLLAMA_NUM_PARALLEL=4
export OLLAMA_MAX_LOADED_MODELS=2

# Restart Ollama with new settings
pkill ollama
ollama serve &
```

#### 3. Memory Optimization
```bash
# Configure memory usage
export OLLAMA_GPU_OVERHEAD=0.1      # Reserve 10% GPU memory
export OLLAMA_MAX_VRAM=0.8          # Use max 80% VRAM

# For CPU-only systems
export OLLAMA_NUM_THREADS=$(nproc)
export OLLAMA_MAX_LOADED_MODELS=1   # Reduce for limited RAM
```

---

## 🎮 GPU Acceleration

### NVIDIA GPU Setup

#### 1. Prerequisites
```bash
# Check NVIDIA GPU
nvidia-smi

# Install NVIDIA Container Toolkit (for Docker)
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | sudo tee /etc/apt/sources.list.d/nvidia-docker.list
sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit
```

#### 2. Ollama GPU Configuration
```bash
# Verify GPU detection
ollama run llama3.2:3b "Test GPU" --verbose

# Check GPU usage during inference
watch -n 1 nvidia-smi

# Configure GPU memory
export CUDA_VISIBLE_DEVICES=0      # Use first GPU
export OLLAMA_GPU_OVERHEAD=0.1     # Reserve memory
```

#### 3. Multi-GPU Setup
```bash
# Use multiple GPUs
export CUDA_VISIBLE_DEVICES=0,1

# Load balance across GPUs
export OLLAMA_NUM_PARALLEL=8       # Increase parallel requests
export OLLAMA_MAX_LOADED_MODELS=4  # Load models on different GPUs
```

### AMD GPU Setup (ROCm)

#### 1. Install ROCm
```bash
# Ubuntu/Debian
wget -q -O - https://repo.radeon.com/rocm/rocm.gpg.key | sudo apt-key add -
echo 'deb [arch=amd64] https://repo.radeon.com/rocm/apt/debian/ ubuntu main' | sudo tee /etc/apt/sources.list.d/rocm.list
sudo apt update && sudo apt install rocm-dev rocm-smi

# Set environment
export ROCM_PATH=/opt/rocm
export HSA_OVERRIDE_GFX_VERSION=10.3.0  # Adjust for your GPU
```

#### 2. Configure Ollama for ROCm
```bash
# Verify ROCm detection
rocm-smi

# Run Ollama with ROCm
HSA_OVERRIDE_GFX_VERSION=10.3.0 ollama serve
```

### Intel GPU Setup

#### 1. Install Intel GPU drivers
```bash
# Ubuntu/Debian
sudo apt install intel-gpu-tools

# Verify Intel GPU
intel_gpu_top
```

#### 2. Configure for Intel GPU
```bash
# Use Intel GPU acceleration (experimental)
export OLLAMA_GPU_TYPE=intel
ollama serve
```

---

## 🔧 Advanced Configuration

### Custom Model Creation

#### 1. Create Custom Modelfile
```bash
# Create a Modelfile for meeting summarization
cat > Modelfile << EOF
FROM llama3.2:3b

# Set custom parameters
PARAMETER temperature 0.7
PARAMETER top_p 0.9
PARAMETER top_k 40
PARAMETER repeat_penalty 1.1

# Custom system prompt for meeting summarization
SYSTEM """
You are an expert meeting assistant. Your task is to create concise, well-structured summaries of meeting transcripts. Focus on:

1. Key decisions made
2. Action items with owners
3. Important discussion points
4. Next steps and deadlines

Format your response in clear sections with bullet points. Be objective and capture the essential information without adding interpretation.
"""

# Set custom template
TEMPLATE """{{ if .System }}<|start_header_id|>system<|end_header_id|>

{{ .System }}<|eot_id|>{{ end }}{{ if .Prompt }}<|start_header_id|>user<|end_header_id|>

{{ .Prompt }}<|eot_id|>{{ end }}<|start_header_id|>assistant<|end_header_id|>

{{ .Response }}<|eot_id|>"""
EOF

# Create the custom model
ollama create meetily-summarizer -f Modelfile

# Test the custom model
ollama run meetily-summarizer "Summarize this meeting transcript: [transcript content]"
```

#### 2. Model Fine-tuning (Advanced)
```bash
# Create training data format
cat > training_data.jsonl << EOF
{"prompt": "Summarize this meeting:", "completion": "Meeting Summary:\n\n**Key Decisions:**\n- Decision 1\n- Decision 2\n\n**Action Items:**\n- Task 1 (Owner: John)\n- Task 2 (Owner: Jane)"}
EOF

# Note: Fine-tuning requires additional tools and is beyond basic setup
```

### API Integration Patterns

#### 1. Streaming Responses
```python
# backend/app/ollama_streaming.py
import requests
import json

def stream_ollama_response(prompt, model="llama3.2:3b"):
    """Stream response from Ollama for real-time updates"""
    url = "http://localhost:11434/api/generate"
    data = {
        "model": model,
        "prompt": prompt,
        "stream": True
    }
    
    response = requests.post(url, json=data, stream=True)
    
    for line in response.iter_lines():
        if line:
            chunk = json.loads(line)
            if not chunk.get('done', False):
                yield chunk.get('response', '')
            else:
                break
```

#### 2. Batch Processing
```python
# backend/app/ollama_batch.py
import asyncio
import aiohttp

async def process_multiple_transcripts(transcripts, model="llama3.2:3b"):
    """Process multiple transcripts concurrently"""
    async with aiohttp.ClientSession() as session:
        tasks = []
        for transcript in transcripts:
            task = process_single_transcript(session, transcript, model)
            tasks.append(task)
        
        results = await asyncio.gather(*tasks)
        return results

async def process_single_transcript(session, transcript, model):
    """Process a single transcript"""
    url = "http://localhost:11434/api/generate"
    data = {
        "model": model,
        "prompt": f"Summarize this meeting transcript: {transcript}",
        "stream": False
    }
    
    async with session.post(url, json=data) as response:
        result = await response.json()
        return result.get('response', '')
```

### Load Balancing and High Availability

#### 1. Multiple Ollama Instances
```bash
# Run multiple Ollama instances on different ports
OLLAMA_HOST=0.0.0.0:11434 ollama serve &
OLLAMA_HOST=0.0.0.0:11435 ollama serve &
OLLAMA_HOST=0.0.0.0:11436 ollama serve &

# Configure load balancer (nginx example)
cat > /etc/nginx/conf.d/ollama.conf << EOF
upstream ollama_backend {
    server localhost:11434;
    server localhost:11435;
    server localhost:11436;
}

server {
    listen 11434;
    location / {
        proxy_pass http://ollama_backend;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
    }
}
EOF
```

#### 2. Health Monitoring
```bash
# Create health check script
cat > ollama_health_check.sh << EOF
#!/bin/bash

check_ollama() {
    local port=\$1
    local response=\$(curl -s -w "%{http_code}" -o /dev/null http://localhost:\$port/api/tags)
    if [ "\$response" = "200" ]; then
        echo "Ollama on port \$port: OK"
        return 0
    else
        echo "Ollama on port \$port: FAILED"
        return 1
    fi
}

# Check all instances
check_ollama 11434
check_ollama 11435
check_ollama 11436
EOF

chmod +x ollama_health_check.sh

# Run health checks periodically
while true; do
    ./ollama_health_check.sh
    sleep 60
done &
```

---

## 🔍 Troubleshooting

### Common Issues and Solutions

#### 1. Ollama Service Issues

**Issue**: Ollama service won't start
```bash
# Check service status
systemctl status ollama

# Check logs
journalctl -u ollama -f

# Common fixes:
sudo systemctl restart ollama
sudo systemctl enable ollama

# Check port availability
netstat -tlnp | grep 11434
```

**Issue**: Permission denied errors
```bash
# Fix ownership
sudo chown -R ollama:ollama /usr/share/ollama

# Fix permissions
sudo chmod 755 /usr/local/bin/ollama

# Add user to ollama group
sudo usermod -a -G ollama $USER
```

#### 2. Model Download Issues

**Issue**: Model download fails or is slow
```bash
# Check disk space
df -h

# Check internet connection
curl -I https://ollama.ai

# Download with retry
for i in {1..3}; do
    ollama pull llama3.2:3b && break
    echo "Retry $i failed, waiting..."
    sleep 10
done

# Manual download (if needed)
wget https://huggingface.co/microsoft/DialoGPT-medium/resolve/main/pytorch_model.bin
```

**Issue**: Model corruption
```bash
# Remove corrupted model
ollama rm llama3.2:3b

# Clear cache
rm -rf ~/.ollama/models/blobs/*

# Re-download
ollama pull llama3.2:3b
```

#### 3. Performance Issues

**Issue**: Slow inference speed
```bash
# Check system resources
htop
nvidia-smi  # For GPU systems

# Optimize model selection
ollama pull llama3.2:3b-q4_0  # Smaller quantized version

# Increase parallel processing
export OLLAMA_NUM_PARALLEL=8
systemctl restart ollama
```

**Issue**: High memory usage
```bash
# Monitor memory
watch -n 1 'free -h && echo "---" && ps aux | grep ollama'

# Reduce loaded models
export OLLAMA_MAX_LOADED_MODELS=1

# Use smaller models
ollama pull phi3:mini
```

#### 4. Integration Issues

**Issue**: Meetily can't connect to Ollama
```bash
# Test connection
curl http://localhost:11434/api/tags

# Check firewall
sudo ufw status
sudo ufw allow 11434

# Check Meetily configuration
cat backend/.env | grep OLLAMA

# Test from Meetily backend
cd backend
python3 -c "
import requests
response = requests.get('http://localhost:11434/api/tags')
print(f'Status: {response.status_code}')
print(f'Response: {response.text}')
"
```

**Issue**: Timeout errors
```bash
# Increase timeout in Meetily
# Edit backend/.env
OLLAMA_TIMEOUT=600  # 10 minutes

# Increase Ollama timeout
export OLLAMA_REQUEST_TIMEOUT=600
systemctl restart ollama
```

### Debugging Tools

#### 1. Ollama Debug Mode
```bash
# Run Ollama with debug logging
OLLAMA_DEBUG=1 ollama serve

# Check detailed logs
tail -f ~/.ollama/logs/server.log
```

#### 2. API Testing
```bash
# Test API endpoints
curl -X GET http://localhost:11434/api/tags
curl -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{"model": "llama3.2:3b", "prompt": "Hello", "stream": false}'

# Test with verbose output
curl -v -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{"model": "llama3.2:3b", "prompt": "Test", "stream": false}'
```

#### 3. Performance Monitoring
```bash
# Monitor Ollama performance
watch -n 1 'curl -s http://localhost:11434/api/ps'

# Monitor system resources
htop
iotop
nvidia-smi -l 1  # For NVIDIA GPUs
```

---

## 📚 Best Practices

### Security Best Practices

#### 1. Network Security
```bash
# Bind to localhost only (production)
export OLLAMA_HOST=127.0.0.1:11434

# Use firewall rules
sudo ufw deny 11434
sudo ufw allow from 127.0.0.1 to any port 11434

# Use reverse proxy with authentication
# (nginx/apache configuration)
```

#### 2. Access Control
```bash
# Create dedicated user
sudo useradd -r -s /bin/false ollama-user

# Run Ollama as non-root
sudo -u ollama-user ollama serve

# Restrict file permissions
chmod 600 ~/.ollama/config.json
```

### Performance Best Practices

#### 1. Model Selection Strategy
```bash
# Development: Use fast models
ollama pull phi3:mini

# Testing: Use balanced models  
ollama pull llama3.2:3b

# Production: Use quality models
ollama pull llama3.2:7b

# Specialized: Use domain-specific models
ollama pull codellama:7b  # For technical meetings
```

#### 2. Resource Management
```bash
# Monitor and limit resources
# Set memory limits
ulimit -v 16777216  # 16GB virtual memory limit

# Set CPU limits
cpulimit -l 800 -p $(pgrep ollama)  # Limit to 8 cores

# Use cgroups for better control
sudo systemctl set-property ollama.service MemoryMax=16G
sudo systemctl set-property ollama.service CPUQuota=800%
```

### Maintenance Best Practices

#### 1. Regular Updates
```bash
# Update Ollama
curl -fsSL https://ollama.ai/install.sh | sh

# Update models
ollama pull llama3.2:3b  # Re-download latest version

# Clean up old models
ollama list | grep -v "NAME" | awk '{print $1}' | xargs -I {} ollama rm {}
```

#### 2. Backup and Recovery
```bash
# Backup Ollama models
tar -czf ollama-models-backup.tar.gz ~/.ollama/models/

# Backup configuration
cp ~/.ollama/config.json ollama-config-backup.json

# Restore from backup
tar -xzf ollama-models-backup.tar.gz -C ~/
cp ollama-config-backup.json ~/.ollama/config.json
```

#### 3. Monitoring and Logging
```bash
# Set up log rotation
sudo tee /etc/logrotate.d/ollama > /dev/null <<EOF
/var/log/ollama/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 ollama ollama
    postrotate
        systemctl reload ollama
    endscript
}
EOF

# Monitor disk usage
du -sh ~/.ollama/models/
```

---

## 🎯 Integration Examples

### Complete Meetily Integration

#### 1. Backend Integration Code
```python
# backend/app/ollama_client.py
import asyncio
import aiohttp
import json
from typing import Optional, Dict, Any
import logging

logger = logging.getLogger(__name__)

class OllamaClient:
    def __init__(self, host: str = "http://localhost:11434", timeout: int = 300):
        self.host = host
        self.timeout = timeout
        
    async def generate_summary(self, transcript: str, model: str = "llama3.2:3b") -> Optional[str]:
        """Generate meeting summary using Ollama"""
        prompt = f"""
        Please analyze this meeting transcript and provide a structured summary:

        {transcript}

        Format your response as follows:
        ## Key Decisions
        - [List key decisions made]

        ## Action Items  
        - [List action items with owners if mentioned]

        ## Discussion Points
        - [List main topics discussed]

        ## Next Steps
        - [List next steps and deadlines]
        """
        
        try:
            async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=self.timeout)) as session:
                data = {
                    "model": model,
                    "prompt": prompt,
                    "stream": False,
                    "options": {
                        "temperature": 0.7,
                        "top_p": 0.9,
                        "top_k": 40
                    }
                }
                
                async with session.post(f"{self.host}/api/generate", json=data) as response:
                    if response.status == 200:
                        result = await response.json()
                        return result.get("response", "")
                    else:
                        logger.error(f"Ollama API error: {response.status}")
                        return None
                        
        except asyncio.TimeoutError:
            logger.error("Ollama request timed out")
            return None
        except Exception as e:
            logger.error(f"Ollama client error: {str(e)}")
            return None
    
    async def health_check(self) -> bool:
        """Check if Ollama is healthy"""
        try:
            async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=5)) as session:
                async with session.get(f"{self.host}/api/tags") as response:
                    return response.status == 200
        except:
            return False
```

#### 2. Configuration Management
```python
# backend/app/config.py
import os
from typing import Dict, Any

class OllamaConfig:
    def __init__(self):
        self.host = os.getenv("OLLAMA_HOST", "http://localhost:11434")
        self.model = os.getenv("OLLAMA_MODEL", "llama3.2:3b")
        self.timeout = int(os.getenv("OLLAMA_TIMEOUT", "300"))
        self.temperature = float(os.getenv("OLLAMA_TEMPERATURE", "0.7"))
        self.max_tokens = int(os.getenv("OLLAMA_MAX_TOKENS", "2048"))
        
    def get_generation_params(self) -> Dict[str, Any]:
        return {
            "temperature": self.temperature,
            "top_p": 0.9,
            "top_k": 40,
            "repeat_penalty": 1.1,
            "num_predict": self.max_tokens
        }
```

#### 3. Error Handling and Fallbacks
```python
# backend/app/ai_service.py
from .ollama_client import OllamaClient
from .cloud_clients import ClaudeClient, GroqClient
import logging

logger = logging.getLogger(__name__)

class AIService:
    def __init__(self):
        self.ollama = OllamaClient()
        self.claude = ClaudeClient()
        self.groq = GroqClient()
        
    async def generate_summary(self, transcript: str) -> str:
        """Generate summary with fallback providers"""
        
        # Try Ollama first (local)
        if await self.ollama.health_check():
            logger.info("Using Ollama for summary generation")
            summary = await self.ollama.generate_summary(transcript)
            if summary:
                return summary
            logger.warning("Ollama failed, trying fallback")
        
        # Fallback to cloud providers
        logger.info("Using cloud fallback for summary generation")
        
        # Try Groq (fast)
        try:
            return await self.groq.generate_summary(transcript)
        except Exception as e:
            logger.error(f"Groq failed: {e}")
        
        # Try Claude (high quality)
        try:
            return await self.claude.generate_summary(transcript)
        except Exception as e:
            logger.error(f"Claude failed: {e}")
        
        # Final fallback
        return "Summary generation failed. Please check your AI service configuration."
```

---

## 🎉 Conclusion

You now have a comprehensive Ollama integration with Meetily that provides:

- **Complete Privacy**: All AI processing happens locally
- **High Performance**: Optimized for your hardware
- **Reliability**: Fallback options for high availability
- **Flexibility**: Multiple models for different use cases
- **Scalability**: Can handle multiple concurrent requests

### Next Steps

1. **Test the Integration**: Run a complete meeting workflow
2. **Optimize Performance**: Tune models and parameters for your use case
3. **Monitor Usage**: Set up logging and monitoring
4. **Scale as Needed**: Add more models or instances based on usage

For additional support, refer to the [main installation guide](LINUX_INSTALLATION_GUIDE.md) or reach out to the community.