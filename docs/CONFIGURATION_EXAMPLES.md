# Meetily Configuration Examples

This document provides comprehensive configuration examples for different environments and use cases when setting up Meetily on Linux systems.

## Table of Contents

1. [Environment Configuration](#environment-configuration)
2. [Backend Configuration](#backend-configuration)
3. [Frontend Configuration](#frontend-configuration)
4. [Ollama Configuration](#ollama-configuration)
5. [GPU Configuration](#gpu-configuration)
6. [Audio Configuration](#audio-configuration)
7. [Database Configuration](#database-configuration)
8. [Security Configuration](#security-configuration)
9. [Performance Optimization](#performance-optimization)
10. [Docker Configuration](#docker-configuration)
11. [Systemd Service Configuration](#systemd-service-configuration)
12. [Development Environment](#development-environment)
13. [Production Environment](#production-environment)

## Environment Configuration

### Development Environment Variables

Create `frontend/.env.local` for development:

```bash
# Frontend Development Configuration
NEXT_PUBLIC_API_URL=http://localhost:8000
NEXT_PUBLIC_OLLAMA_URL=http://localhost:11434
NEXT_PUBLIC_ENVIRONMENT=development
NEXT_PUBLIC_DEBUG=true

# Tauri Development
TAURI_DEV_HOST=localhost
TAURI_DEV_PORT=3118

# Optional: Enable verbose logging
RUST_LOG=debug
TAURI_DEBUG=true
```

### Production Environment Variables

Create `frontend/.env.production`:

```bash
# Frontend Production Configuration
NEXT_PUBLIC_API_URL=https://api.yourdomain.com
NEXT_PUBLIC_OLLAMA_URL=http://localhost:11434
NEXT_PUBLIC_ENVIRONMENT=production
NEXT_PUBLIC_DEBUG=false

# Performance optimizations
NEXT_PUBLIC_ENABLE_ANALYTICS=true
NEXT_PUBLIC_CACHE_DURATION=3600
```

### Backend Environment Variables

Create `backend/.env`:

```bash
# Backend Configuration
DATABASE_URL=sqlite:///home/user/.local/share/meetily/meetily.db
API_HOST=0.0.0.0
API_PORT=8000
DEBUG=false

# CORS Configuration
ALLOWED_ORIGINS=["http://localhost:3000", "http://localhost:3118", "https://yourdomain.com"]

# LLM API Keys (choose your providers)
OPENAI_API_KEY=your_openai_api_key_here
ANTHROPIC_API_KEY=your_anthropic_api_key_here
GROQ_API_KEY=your_groq_api_key_here
GEMINI_API_KEY=your_gemini_api_key_here

# Ollama Configuration
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_DEFAULT_MODEL=llama2:7b
OLLAMA_TIMEOUT=300

# Audio Processing
MAX_AUDIO_FILE_SIZE=100MB
AUDIO_SAMPLE_RATE=16000
AUDIO_CHANNELS=1

# Security
SECRET_KEY=your_secret_key_here_change_this_in_production
JWT_ALGORITHM=HS256
JWT_EXPIRATION_HOURS=24

# Performance
MAX_WORKERS=4
CHUNK_SIZE=1024
ENABLE_GPU=true
```

## Backend Configuration

### FastAPI Configuration

Create `backend/config/settings.py`:

```python
from pydantic import BaseSettings
from typing import List, Optional
import os

class Settings(BaseSettings):
    # API Configuration
    api_host: str = "0.0.0.0"
    api_port: int = 8000
    debug: bool = False
    
    # Database
    database_url: str = "sqlite:///~/.local/share/meetily/meetily.db"
    
    # CORS
    allowed_origins: List[str] = [
        "http://localhost:3000",
        "http://localhost:3118",
        "https://yourdomain.com"
    ]
    
    # LLM Providers
    openai_api_key: Optional[str] = None
    anthropic_api_key: Optional[str] = None
    groq_api_key: Optional[str] = None
    gemini_api_key: Optional[str] = None
    
    # Ollama
    ollama_base_url: str = "http://localhost:11434"
    ollama_default_model: str = "llama2:7b"
    ollama_timeout: int = 300
    
    # Audio Processing
    max_audio_file_size: str = "100MB"
    audio_sample_rate: int = 16000
    audio_channels: int = 1
    
    # Security
    secret_key: str = "change-this-in-production"
    jwt_algorithm: str = "HS256"
    jwt_expiration_hours: int = 24
    
    # Performance
    max_workers: int = 4
    chunk_size: int = 1024
    enable_gpu: bool = True
    
    class Config:
        env_file = ".env"
        case_sensitive = False

settings = Settings()
```

### Database Configuration

Create `backend/config/database.py`:

```python
from sqlalchemy import create_engine, MetaData
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.orm import sessionmaker
from sqlalchemy.pool import StaticPool
import os

# Ensure database directory exists
db_dir = os.path.expanduser("~/.local/share/meetily")
os.makedirs(db_dir, exist_ok=True)

# Database URL
DATABASE_URL = f"sqlite:///{db_dir}/meetily.db"

# SQLite-specific configuration for better performance
engine = create_engine(
    DATABASE_URL,
    connect_args={
        "check_same_thread": False,
        "timeout": 30,
    },
    poolclass=StaticPool,
    echo=False,  # Set to True for SQL debugging
)

# Configure SQLite for better performance
@event.listens_for(engine, "connect")
def set_sqlite_pragma(dbapi_connection, connection_record):
    cursor = dbapi_connection.cursor()
    # Enable WAL mode for better concurrency
    cursor.execute("PRAGMA journal_mode=WAL")
    # Increase cache size (in KB)
    cursor.execute("PRAGMA cache_size=10000")
    # Enable foreign key constraints
    cursor.execute("PRAGMA foreign_keys=ON")
    # Optimize for speed
    cursor.execute("PRAGMA synchronous=NORMAL")
    cursor.execute("PRAGMA temp_store=MEMORY")
    cursor.close()

SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)
Base = declarative_base()
```

## Frontend Configuration

### Next.js Configuration

Update `frontend/next.config.js`:

```javascript
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  images: {
    unoptimized: true
  },
  
  // Environment-specific configuration
  env: {
    CUSTOM_KEY: process.env.CUSTOM_KEY,
  },
  
  // Performance optimizations
  experimental: {
    optimizeCss: true,
    optimizePackageImports: ['@tauri-apps/api'],
  },
  
  // Security headers
  async headers() {
    return [
      {
        source: '/(.*)',
        headers: [
          {
            key: 'X-Frame-Options',
            value: 'DENY'
          },
          {
            key: 'X-Content-Type-Options',
            value: 'nosniff'
          },
          {
            key: 'Referrer-Policy',
            value: 'strict-origin-when-cross-origin'
          }
        ]
      }
    ]
  },
  
  // Webpack configuration for Tauri
  webpack: (config, { isServer }) => {
    if (!isServer) {
      config.resolve.fallback = {
        ...config.resolve.fallback,
        fs: false,
        net: false,
        tls: false,
      };
    }
    return config;
  },
};

module.exports = nextConfig;
```

### Tauri Configuration

Create `frontend/src-tauri/tauri.dev.conf.json` for development:

```json
{
  "$schema": "../node_modules/@tauri-apps/cli/config.schema.json",
  "productName": "meetily-dev",
  "version": "0.2.0-dev",
  "identifier": "com.meetily.ai.dev",
  "build": {
    "frontendDist": "../out",
    "devUrl": "http://localhost:3118",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "Meetily (Development)",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false,
        "theme": "Light",
        "decorations": true,
        "center": true
      }
    ],
    "security": {
      "csp": {
        "default-src": "'self'",
        "style-src": "'self' 'unsafe-inline'",
        "img-src": "'self' asset: https://asset.localhost data:",
        "connect-src": "'self' http://localhost:* ws://localhost:*"
      }
    }
  }
}
```

## Ollama Configuration

### Basic Ollama Configuration

Create `~/.ollama/config.json`:

```json
{
  "host": "0.0.0.0:11434",
  "models_path": "/home/user/.ollama/models",
  "max_loaded_models": 3,
  "gpu_memory_fraction": 0.8,
  "log_level": "info",
  "timeout": 300,
  "keep_alive": "5m"
}
```

### GPU-Optimized Ollama Configuration

For systems with GPU acceleration:

```json
{
  "host": "0.0.0.0:11434",
  "models_path": "/home/user/.ollama/models",
  "max_loaded_models": 2,
  "gpu_memory_fraction": 0.9,
  "gpu_layers": -1,
  "log_level": "info",
  "timeout": 600,
  "keep_alive": "10m",
  "num_ctx": 4096,
  "num_gpu": 1
}
```

### Ollama Environment Variables

Create `/etc/systemd/system/ollama.service.d/override.conf`:

```ini
[Service]
Environment="OLLAMA_HOST=0.0.0.0:11434"
Environment="OLLAMA_MODELS=/home/user/.ollama/models"
Environment="OLLAMA_MAX_LOADED_MODELS=3"
Environment="OLLAMA_GPU_MEMORY_FRACTION=0.8"
Environment="OLLAMA_NUM_PARALLEL=2"
Environment="OLLAMA_FLASH_ATTENTION=1"
Environment="CUDA_VISIBLE_DEVICES=0"
```

## GPU Configuration

### NVIDIA GPU Configuration

Create `~/.meetily/gpu.conf`:

```ini
[nvidia]
enabled=true
device_id=0
memory_fraction=0.8
compute_capability=auto

[cuda]
version=12.0
cudnn_version=8.9
cublas_enabled=true

[performance]
mixed_precision=true
tensor_cores=true
memory_pool=true
```

### AMD GPU Configuration (ROCm)

```ini
[amd]
enabled=true
device_id=0
memory_fraction=0.8
rocm_version=5.7

[hip]
visible_devices=0
force_device=0

[performance]
mixed_precision=false
memory_pool=true
```

### Intel GPU Configuration

```ini
[intel]
enabled=true
device_id=0
memory_fraction=0.7
level_zero=true

[opencl]
platform=intel
device_type=gpu

[performance]
fp16_enabled=true
memory_pool=false
```

## Audio Configuration

### ALSA Configuration

Create `~/.asoundrc`:

```
pcm.!default {
    type pulse
}

ctl.!default {
    type pulse
}

# High-quality capture device
pcm.meetily_capture {
    type hw
    card 0
    device 0
    rate 48000
    channels 2
    format S16_LE
}

# Low-latency playback
pcm.meetily_playback {
    type hw
    card 0
    device 0
    rate 48000
    channels 2
    format S16_LE
    period_size 256
    buffer_size 1024
}
```

### PulseAudio Configuration

Create `~/.pulse/daemon.conf`:

```ini
# High-quality audio processing
default-sample-format = s16le
default-sample-rate = 48000
default-sample-channels = 2

# Low-latency settings
default-fragments = 4
default-fragment-size-msec = 5

# Buffer settings for recording
high-priority = yes
nice-level = -11
realtime-scheduling = yes
realtime-priority = 5

# Resample quality
resample-method = speex-float-10
avoid-resampling = yes

# Module loading
load-module module-udev-detect
load-module module-native-protocol-unix
load-module module-default-device-restore
```

### Audio Device Configuration

Create `~/.meetily/audio.conf`:

```ini
[input]
device=default
sample_rate=16000
channels=1
format=s16le
buffer_size=1024

[processing]
noise_reduction=true
auto_gain=true
echo_cancellation=false
voice_activity_detection=true

[output]
device=default
sample_rate=48000
channels=2
format=s16le
```

## Database Configuration

### SQLite Optimization

Create `~/.meetily/database.conf`:

```ini
[sqlite]
journal_mode=WAL
synchronous=NORMAL
cache_size=10000
temp_store=MEMORY
mmap_size=268435456
page_size=4096

[backup]
enabled=true
interval=24h
retention=7d
location=/home/user/.local/share/meetily/backups

[maintenance]
auto_vacuum=incremental
analyze_threshold=1000
optimize_interval=7d
```

### Database Connection Pool

```python
# backend/config/database_pool.py
from sqlalchemy.pool import QueuePool
from sqlalchemy import create_engine

engine = create_engine(
    DATABASE_URL,
    poolclass=QueuePool,
    pool_size=10,
    max_overflow=20,
    pool_pre_ping=True,
    pool_recycle=3600,
    connect_args={
        "check_same_thread": False,
        "timeout": 30,
    }
)
```

## Security Configuration

### SSL/TLS Configuration

Create `backend/config/ssl.conf`:

```nginx
# For reverse proxy setup
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
ssl_prefer_server_ciphers off;
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 10m;

# Security headers
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
add_header X-Frame-Options DENY always;
add_header X-Content-Type-Options nosniff always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
```

### API Security Configuration

```python
# backend/config/security.py
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.trustedhost import TrustedHostMiddleware
from fastapi.middleware.httpsredirect import HTTPSRedirectMiddleware

def configure_security(app: FastAPI):
    # HTTPS redirect in production
    if not settings.debug:
        app.add_middleware(HTTPSRedirectMiddleware)
    
    # Trusted hosts
    app.add_middleware(
        TrustedHostMiddleware,
        allowed_hosts=["localhost", "127.0.0.1", "yourdomain.com"]
    )
    
    # CORS configuration
    app.add_middleware(
        CORSMiddleware,
        allow_origins=settings.allowed_origins,
        allow_credentials=True,
        allow_methods=["GET", "POST", "PUT", "DELETE"],
        allow_headers=["*"],
        expose_headers=["X-Total-Count"]
    )
```

## Performance Optimization

### System Performance Configuration

Create `/etc/sysctl.d/99-meetily.conf`:

```ini
# Network optimizations
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216

# File system optimizations
fs.file-max = 65536
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5

# Process limits
kernel.pid_max = 65536
```

### Application Performance Configuration

Create `~/.meetily/performance.conf`:

```ini
[cpu]
max_threads=8
thread_pool_size=4
cpu_affinity=auto

[memory]
max_heap_size=4G
gc_threshold=0.8
buffer_pool_size=512M

[io]
async_io=true
io_threads=2
read_buffer_size=64K
write_buffer_size=64K

[cache]
enabled=true
max_size=1G
ttl=3600
compression=true
```
## Docker Configuration

### Development Docker Compose

Create `docker-compose.dev.yml`:

```yaml
version: '3.8'

services:
  meetily-backend:
    build:
      context: ./backend
      dockerfile: Dockerfile.dev
    ports:
      - "8000:8000"
    volumes:
      - ./backend:/app
      - ~/.local/share/meetily:/data
    environment:
      - DEBUG=true
      - DATABASE_URL=sqlite:///data/meetily.db
      - OLLAMA_BASE_URL=http://ollama:11434
    depends_on:
      - ollama
    restart: unless-stopped

  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ~/.ollama:/root/.ollama
    environment:
      - OLLAMA_HOST=0.0.0.0:11434
      - OLLAMA_MAX_LOADED_MODELS=2
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
    restart: unless-stopped

  meetily-frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile.dev
    ports:
      - "3000:3000"
    volumes:
      - ./frontend:/app
      - /app/node_modules
    environment:
      - NEXT_PUBLIC_API_URL=http://localhost:8000
      - NEXT_PUBLIC_OLLAMA_URL=http://localhost:11434
    depends_on:
      - meetily-backend
    restart: unless-stopped

volumes:
  ollama_data:
  meetily_data:
```

### Production Docker Compose

Create `docker-compose.prod.yml`:

```yaml
version: '3.8'

services:
  meetily-backend:
    build:
      context: ./backend
      dockerfile: Dockerfile.prod
    ports:
      - "8000:8000"
    volumes:
      - meetily_data:/data
      - ./logs:/app/logs
    environment:
      - DEBUG=false
      - DATABASE_URL=sqlite:///data/meetily.db
      - OLLAMA_BASE_URL=http://ollama:11434
      - SECRET_KEY=${SECRET_KEY}
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    depends_on:
      - ollama
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  ollama:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    environment:
      - OLLAMA_HOST=0.0.0.0:11434
      - OLLAMA_MAX_LOADED_MODELS=1
      - OLLAMA_GPU_MEMORY_FRACTION=0.8
    deploy:
      resources:
        limits:
          memory: 8G
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:11434/api/version"]
      interval: 30s
      timeout: 10s
      retries: 3

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
      - ./frontend/out:/usr/share/nginx/html
    depends_on:
      - meetily-backend
    restart: unless-stopped

volumes:
  ollama_data:
  meetily_data:
```

### Backend Dockerfile

Create `backend/Dockerfile.prod`:

```dockerfile
FROM python:3.11-slim

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    curl \
    libasound2-dev \
    libpulse-dev \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy requirements and install Python dependencies
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

# Copy application code
COPY . .

# Create non-root user
RUN useradd -m -u 1000 meetily && chown -R meetily:meetily /app
USER meetily

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# Start application
CMD ["uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000", "--workers", "4"]
```

## Systemd Service Configuration

### Meetily Backend Service

Create `/etc/systemd/system/meetily-backend.service`:

```ini
[Unit]
Description=Meetily Backend API Server
After=network.target ollama.service
Wants=ollama.service

[Service]
Type=simple
User=meetily
Group=meetily
WorkingDirectory=/opt/meetily/backend
Environment=PATH=/opt/meetily/backend/venv/bin
Environment=DATABASE_URL=sqlite:///home/meetily/.local/share/meetily/meetily.db
Environment=OLLAMA_BASE_URL=http://localhost:11434
EnvironmentFile=/etc/meetily/backend.env
ExecStart=/opt/meetily/backend/venv/bin/uvicorn app.main:app --host 0.0.0.0 --port 8000 --workers 4
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=meetily-backend

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/meetily/.local/share/meetily
ReadWritePaths=/tmp

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### Ollama Service Override

Create `/etc/systemd/system/ollama.service.d/override.conf`:

```ini
[Unit]
Description=Ollama LLM Server
After=network.target

[Service]
Type=simple
User=ollama
Group=ollama
WorkingDirectory=/home/ollama
Environment=OLLAMA_HOST=0.0.0.0:11434
Environment=OLLAMA_MODELS=/home/ollama/.ollama/models
Environment=OLLAMA_MAX_LOADED_MODELS=2
Environment=OLLAMA_GPU_MEMORY_FRACTION=0.8
Environment=CUDA_VISIBLE_DEVICES=0
ExecStart=/usr/local/bin/ollama serve
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ollama

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/home/ollama/.ollama

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### Audio System Service

Create `/etc/systemd/system/meetily-audio.service`:

```ini
[Unit]
Description=Meetily Audio Processing Service
After=sound.target pulseaudio.service
Requires=sound.target

[Service]
Type=simple
User=meetily
Group=audio
Environment=PULSE_RUNTIME_PATH=/run/user/1000/pulse
ExecStartPre=/bin/sleep 2
ExecStart=/opt/meetily/bin/audio-processor
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=meetily-audio

# Audio-specific settings
SupplementaryGroups=audio pulse-access
PAMName=login

[Install]
WantedBy=multi-user.target
```

## Development Environment

### Development Setup Script

Create `scripts/setup-dev.sh`:

```bash
#!/bin/bash
set -e

echo "Setting up Meetily development environment..."

# Create development directories
mkdir -p ~/.local/share/meetily/{recordings,models,logs,backups}
mkdir -p ~/.config/meetily

# Backend setup
echo "Setting up backend..."
cd backend
python3 -m venv venv
source venv/bin/activate
pip install --upgrade pip
pip install -r requirements.txt
pip install -r requirements-dev.txt

# Create development database
python -c "from app.db import init_db; init_db()"

# Frontend setup
echo "Setting up frontend..."
cd ../frontend
npm install
npm run build

# Install Tauri CLI
cargo install tauri-cli

# Setup Git hooks
echo "Setting up Git hooks..."
cd ..
cp scripts/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit

# Create development configuration
cat > backend/.env.dev << EOF
DEBUG=true
DATABASE_URL=sqlite:///$(pwd)/dev.db
API_HOST=127.0.0.1
API_PORT=8000
OLLAMA_BASE_URL=http://localhost:11434
RUST_LOG=debug
EOF

cat > frontend/.env.local << EOF
NEXT_PUBLIC_API_URL=http://localhost:8000
NEXT_PUBLIC_OLLAMA_URL=http://localhost:11434
NEXT_PUBLIC_ENVIRONMENT=development
NEXT_PUBLIC_DEBUG=true
RUST_LOG=debug
TAURI_DEBUG=true
EOF

echo "Development environment setup complete!"
echo "Run 'npm run dev' in frontend directory to start development server"
echo "Run 'source venv/bin/activate && uvicorn app.main:app --reload' in backend directory to start API server"
```

### Development Docker Configuration

Create `docker-compose.override.yml`:

```yaml
version: '3.8'

services:
  meetily-backend:
    build:
      target: development
    volumes:
      - ./backend:/app
      - backend_cache:/app/.cache
    environment:
      - WATCHDOG_ENABLED=true
      - HOT_RELOAD=true
    command: uvicorn app.main:app --host 0.0.0.0 --port 8000 --reload

  meetily-frontend:
    volumes:
      - ./frontend:/app
      - frontend_node_modules:/app/node_modules
    environment:
      - FAST_REFRESH=true
      - CHOKIDAR_USEPOLLING=true
    command: npm run dev

volumes:
  backend_cache:
  frontend_node_modules:
```

## Production Environment

### Production Deployment Script

Create `scripts/deploy-prod.sh`:

```bash
#!/bin/bash
set -e

echo "Deploying Meetily to production..."

# Configuration
DEPLOY_USER="meetily"
DEPLOY_PATH="/opt/meetily"
SERVICE_NAME="meetily-backend"
BACKUP_PATH="/opt/meetily/backups"

# Create backup
echo "Creating backup..."
sudo systemctl stop $SERVICE_NAME
sudo -u $DEPLOY_USER sqlite3 ~/.local/share/meetily/meetily.db ".backup $BACKUP_PATH/meetily-$(date +%Y%m%d-%H%M%S).db"

# Update code
echo "Updating application..."
cd $DEPLOY_PATH
sudo -u $DEPLOY_USER git pull origin main

# Update backend dependencies
echo "Updating backend..."
cd backend
sudo -u $DEPLOY_USER source venv/bin/activate
sudo -u $DEPLOY_USER pip install -r requirements.txt

# Update frontend
echo "Building frontend..."
cd ../frontend
sudo -u $DEPLOY_USER npm ci --production
sudo -u $DEPLOY_USER npm run build

# Database migrations
echo "Running database migrations..."
cd ../backend
sudo -u $DEPLOY_USER source venv/bin/activate
sudo -u $DEPLOY_USER python -c "from app.db import upgrade_db; upgrade_db()"

# Restart services
echo "Restarting services..."
sudo systemctl start $SERVICE_NAME
sudo systemctl restart nginx

# Health check
echo "Performing health check..."
sleep 10
if curl -f http://localhost:8000/health; then
    echo "Deployment successful!"
else
    echo "Health check failed, rolling back..."
    sudo systemctl stop $SERVICE_NAME
    # Restore backup logic here
    exit 1
fi
```

### Production Environment Variables

Create `/etc/meetily/backend.env`:

```bash
# Production Backend Configuration
DEBUG=false
DATABASE_URL=sqlite:///home/meetily/.local/share/meetily/meetily.db
API_HOST=0.0.0.0
API_PORT=8000

# Security
SECRET_KEY=your_production_secret_key_here
JWT_ALGORITHM=HS256
JWT_EXPIRATION_HOURS=24

# CORS (restrict to your domain)
ALLOWED_ORIGINS=["https://yourdomain.com"]

# LLM API Keys
OPENAI_API_KEY=your_production_openai_key
ANTHROPIC_API_KEY=your_production_anthropic_key

# Ollama
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_DEFAULT_MODEL=llama2:7b
OLLAMA_TIMEOUT=600

# Performance
MAX_WORKERS=8
ENABLE_GPU=true
CHUNK_SIZE=2048

# Logging
LOG_LEVEL=INFO
LOG_FILE=/var/log/meetily/backend.log
```

### Nginx Production Configuration

Create `/etc/nginx/sites-available/meetily`:

```nginx
upstream meetily_backend {
    server 127.0.0.1:8000;
    keepalive 32;
}

server {
    listen 80;
    server_name yourdomain.com www.yourdomain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com www.yourdomain.com;

    # SSL Configuration
    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

    # Security Headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options DENY always;
    add_header X-Content-Type-Options nosniff always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Frontend static files
    location / {
        root /opt/meetily/frontend/out;
        try_files $uri $uri/ /index.html;
        
        # Cache static assets
        location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2)$ {
            expires 1y;
            add_header Cache-Control "public, immutable";
        }
    }

    # API proxy
    location /api/ {
        proxy_pass http://meetily_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # WebSocket support for real-time features
    location /ws/ {
        proxy_pass http://meetily_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # File upload size
    client_max_body_size 100M;

    # Logging
    access_log /var/log/nginx/meetily_access.log;
    error_log /var/log/nginx/meetily_error.log;
}
```

## Configuration Validation

### Configuration Validation Script

Create `scripts/validate-config.sh`:

```bash
#!/bin/bash

echo "=== Meetily Configuration Validation ==="

# Check environment files
echo "Checking environment files..."
for env_file in backend/.env frontend/.env.local; do
    if [ -f "$env_file" ]; then
        echo "✅ $env_file exists"
        # Check for required variables
        if [ "$env_file" = "backend/.env" ]; then
            required_vars=("DATABASE_URL" "SECRET_KEY" "OLLAMA_BASE_URL")
            for var in "${required_vars[@]}"; do
                if grep -q "^$var=" "$env_file"; then
                    echo "✅ $var configured"
                else
                    echo "❌ $var missing in $env_file"
                fi
            done
        fi
    else
        echo "❌ $env_file missing"
    fi
done

# Check database
echo "Checking database..."
if [ -f ~/.local/share/meetily/meetily.db ]; then
    echo "✅ Database file exists"
    if sqlite3 ~/.local/share/meetily/meetily.db ".tables" | grep -q "meetings"; then
        echo "✅ Database schema initialized"
    else
        echo "❌ Database schema not initialized"
    fi
else
    echo "❌ Database file missing"
fi

# Check Ollama
echo "Checking Ollama..."
if command -v ollama >/dev/null 2>&1; then
    echo "✅ Ollama installed"
    if curl -s http://localhost:11434/api/version >/dev/null 2>&1; then
        echo "✅ Ollama service running"
        models=$(ollama list | tail -n +2 | wc -l)
        echo "✅ $models models available"
    else
        echo "❌ Ollama service not running"
    fi
else
    echo "❌ Ollama not installed"
fi

# Check GPU
echo "Checking GPU configuration..."
if command -v nvidia-smi >/dev/null 2>&1; then
    echo "✅ NVIDIA GPU detected"
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
elif command -v rocm-smi >/dev/null 2>&1; then
    echo "✅ AMD GPU detected"
else
    echo "⚠️  No GPU detected (CPU-only mode)"
fi

echo "=== Configuration Validation Complete ==="
```

This comprehensive configuration guide provides examples for all major components and deployment scenarios. Adjust the values according to your specific environment and requirements.