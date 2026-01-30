# 🔨 Build and Deployment Guide for Meetily

This comprehensive guide covers building Meetily from source and deploying it in various environments, from development to production.

## 📋 Table of Contents

- [Development Environment Setup](#-development-environment-setup)
- [Building from Source](#-building-from-source)
- [Production Build Process](#-production-build-process)
- [Docker Deployment](#-docker-deployment)
- [Systemd Service Deployment](#-systemd-service-deployment)
- [Reverse Proxy Setup](#-reverse-proxy-setup)
- [Monitoring and Logging](#-monitoring-and-logging)
- [Backup and Recovery](#-backup-and-recovery)
- [Troubleshooting](#-troubleshooting)

---

## 🛠️ Development Environment Setup

### Prerequisites Verification

Before building, ensure all prerequisites are installed:

```bash
# Check system requirements
./scripts/check-prerequisites.sh

# Or manually verify:
rustc --version    # Should be 1.77+
node --version     # Should be 18+
python3 --version  # Should be 3.8+
cmake --version    # Should be 3.16+
git --version      # Any recent version
```

### Repository Setup

```bash
# Clone your fork
git clone https://github.com/ocobra/meeting-minutes.git
cd meeting-minutes

# Verify repository structure
ls -la
# Should show: frontend/, backend/, docs/, scripts/, etc.

# Check for required files
test -f frontend/package.json && echo "Frontend OK" || echo "Frontend missing"
test -f backend/requirements.txt && echo "Backend OK" || echo "Backend missing"
test -f frontend/src-tauri/Cargo.toml && echo "Tauri OK" || echo "Tauri missing"
```

### Environment Configuration

```bash
# Create development environment file
cat > .env.development << EOF
# Development Configuration
NODE_ENV=development
RUST_LOG=debug
TAURI_DEBUG=true

# Backend Configuration
DATABASE_PATH=./backend/data/meeting_minutes_dev.db
OLLAMA_HOST=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b

# Frontend Configuration
NEXT_PUBLIC_API_URL=http://localhost:5167
NEXT_PUBLIC_WHISPER_URL=http://localhost:8178

# Optional: Cloud API Keys for testing
# ANTHROPIC_API_KEY=your_dev_key
# GROQ_API_KEY=your_dev_key
EOF

# Set proper permissions
chmod 600 .env.development
```

---

## 🏗️ Building from Source

### Quick Build (Recommended)

```bash
# Navigate to frontend directory
cd frontend

# Install dependencies
pnpm install

# Build with automatic GPU detection
./build-gpu.sh

# Expected output location:
# src-tauri/target/release/bundle/appimage/Meetily_0.2.0_amd64.AppImage
```

### Manual Build Process

#### 1. Frontend Dependencies
```bash
cd frontend

# Install Node.js dependencies
pnpm install

# Verify dependencies
pnpm list --depth=0
```

#### 2. Backend Dependencies
```bash
cd ../backend

# Create Python virtual environment
python3 -m venv venv
source venv/bin/activate

# Install Python dependencies
pip install -r requirements.txt

# Verify installation
pip list | grep -E "(fastapi|pydantic|ollama)"
```

#### 3. Rust Dependencies
```bash
cd ../frontend/src-tauri

# Check Rust toolchain
rustup show

# Update if needed
rustup update

# Install additional targets (if needed)
rustup target add x86_64-unknown-linux-gnu
```

#### 4. Build Whisper Server
```bash
cd ../../backend

# Build Whisper server with model
./build_whisper.sh small

# Verify build
ls -la whisper-server-package/
test -f whisper-server-package/run-server.sh && echo "Whisper OK"
```

#### 5. Build Frontend Application
```bash
cd ../frontend

# Development build
pnpm run tauri:dev

# Production build
pnpm run tauri:build

# GPU-specific builds
pnpm run tauri:build:cuda    # NVIDIA
pnpm run tauri:build:vulkan  # AMD/Intel
pnpm run tauri:build:metal   # macOS
```

### Build Verification

```bash
# Check build artifacts
ls -la frontend/src-tauri/target/release/bundle/

# Test the built application
cd frontend/src-tauri/target/release/
./meetily --version

# Verify all components
./meetily --help
```

---

## 🚀 Production Build Process

### Optimized Production Build

#### 1. Environment Preparation
```bash
# Set production environment
export NODE_ENV=production
export RUST_LOG=info
export TAURI_DEBUG=false

# Clean previous builds
cd frontend
rm -rf node_modules/.cache
rm -rf src-tauri/target/release
pnpm cache clean
```

#### 2. Dependency Optimization
```bash
# Install production dependencies only
pnpm install --production --frozen-lockfile

# Audit for vulnerabilities
pnpm audit --audit-level moderate
```

#### 3. Build with Optimizations
```bash
# Build with release optimizations
CARGO_PROFILE_RELEASE_LTO=true \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_PANIC=abort \
./build-gpu.sh

# Strip debug symbols (Linux)
strip src-tauri/target/release/meetily
```

#### 4. Bundle Creation
```bash
# Create AppImage (Linux)
pnpm run tauri:build

# Create additional packages
cd src-tauri/target/release/bundle/

# Create tar.gz archive
tar -czf meetily-linux-x64.tar.gz appimage/

# Create deb package (if configured)
ls -la deb/

# Verify bundle integrity
sha256sum meetily-linux-x64.tar.gz > meetily-linux-x64.tar.gz.sha256
```

### Cross-Platform Builds

#### Linux to Windows (Cross-compilation)
```bash
# Install Windows target
rustup target add x86_64-pc-windows-gnu

# Install cross-compilation tools
sudo apt install gcc-mingw-w64-x86-64

# Build for Windows
cargo build --target x86_64-pc-windows-gnu --release
```

#### Build Matrix
```bash
# Create build script for multiple targets
cat > build-all-platforms.sh << 'EOF'
#!/bin/bash

TARGETS=("x86_64-unknown-linux-gnu" "x86_64-pc-windows-gnu")
FEATURES=("" "cuda" "vulkan")

for target in "${TARGETS[@]}"; do
    for feature in "${FEATURES[@]}"; do
        echo "Building for $target with feature: ${feature:-none}"
        
        if [ -n "$feature" ]; then
            cargo build --target $target --release --features $feature
        else
            cargo build --target $target --release
        fi
        
        if [ $? -eq 0 ]; then
            echo "✅ Build successful: $target ${feature:-none}"
        else
            echo "❌ Build failed: $target ${feature:-none}"
        fi
    done
done
EOF

chmod +x build-all-platforms.sh
./build-all-platforms.sh
```

---

## 🐳 Docker Deployment

### Development Docker Setup

#### 1. Docker Compose for Development
```yaml
# docker-compose.dev.yml
version: '3.8'

services:
  meetily-dev:
    build:
      context: .
      dockerfile: Dockerfile.dev
    ports:
      - "3118:3118"  # Frontend dev server
      - "5167:5167"  # Backend API
      - "8178:8178"  # Whisper server
    volumes:
      - .:/app
      - /app/node_modules
      - /app/target
    environment:
      - NODE_ENV=development
      - RUST_LOG=debug
    command: ["./scripts/dev-start.sh"]

  ollama-dev:
    image: ollama/ollama:latest
    ports:
      - "11434:11434"
    volumes:
      - ollama_dev_data:/root/.ollama
    environment:
      - OLLAMA_HOST=0.0.0.0

volumes:
  ollama_dev_data:
```

#### 2. Development Dockerfile
```dockerfile
# Dockerfile.dev
FROM ubuntu:22.04

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    cmake \
    git \
    nodejs \
    npm \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install pnpm
RUN npm install -g pnpm

# Set working directory
WORKDIR /app

# Copy package files
COPY frontend/package.json frontend/pnpm-lock.yaml ./frontend/
COPY backend/requirements.txt ./backend/

# Install dependencies
RUN cd frontend && pnpm install
RUN cd backend && python3 -m venv venv && \
    . venv/bin/activate && pip install -r requirements.txt

# Copy source code
COPY . .

# Expose ports
EXPOSE 3118 5167 8178

# Start development servers
CMD ["./scripts/dev-start.sh"]
```

### Production Docker Setup

#### 1. Multi-stage Production Dockerfile
```dockerfile
# Dockerfile.prod
# Stage 1: Build frontend
FROM node:18-alpine AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile

COPY frontend/ ./
RUN pnpm run build

# Stage 2: Build Rust components
FROM rust:1.77-slim AS rust-builder

RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY frontend/src-tauri/ ./src-tauri/
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

RUN cd src-tauri && cargo build --release

# Stage 3: Build backend
FROM python:3.11-slim AS backend-builder

WORKDIR /app/backend
COPY backend/requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY backend/ ./

# Stage 4: Final production image
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false -m -d /app meetily

# Copy built applications
COPY --from=rust-builder /app/src-tauri/target/release/meetily /usr/local/bin/
COPY --from=backend-builder /app/backend /app/backend

# Set ownership
RUN chown -R meetily:meetily /app

# Switch to app user
USER meetily
WORKDIR /app

# Expose ports
EXPOSE 5167

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:5167/get-meetings || exit 1

# Start backend
CMD ["python3", "backend/main.py"]
```

#### 2. Production Docker Compose
```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  meetily-app:
    build:
      context: .
      dockerfile: Dockerfile.prod
    restart: unless-stopped
    ports:
      - "5167:5167"
    volumes:
      - meetily_data:/app/data
      - meetily_logs:/app/logs
    environment:
      - NODE_ENV=production
      - DATABASE_PATH=/app/data/meeting_minutes.db
      - OLLAMA_HOST=http://ollama:11434
    depends_on:
      - ollama
    networks:
      - meetily_network

  ollama:
    image: ollama/ollama:latest
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
    networks:
      - meetily_network

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - meetily-app
    networks:
      - meetily_network

volumes:
  meetily_data:
  meetily_logs:
  ollama_data:

networks:
  meetily_network:
    driver: bridge
```

### Docker Deployment Commands

```bash
# Development deployment
docker-compose -f docker-compose.dev.yml up -d

# Production deployment
docker-compose -f docker-compose.prod.yml up -d

# View logs
docker-compose logs -f meetily-app

# Scale services
docker-compose up -d --scale meetily-app=3

# Update deployment
docker-compose pull
docker-compose up -d --force-recreate
```

---

## 🔧 Systemd Service Deployment

### Service Configuration

#### 1. Meetily Backend Service
```ini
# /etc/systemd/system/meetily-backend.service
[Unit]
Description=Meetily Backend Service
After=network-online.target ollama.service
Wants=ollama.service
Requires=network-online.target

[Service]
Type=simple
User=meetily
Group=meetily
WorkingDirectory=/opt/meetily/backend
Environment="PATH=/opt/meetily/venv/bin:/usr/local/bin:/usr/bin:/bin"
Environment="PYTHONPATH=/opt/meetily/backend"
Environment="DATABASE_PATH=/opt/meetily/data/meeting_minutes.db"
Environment="OLLAMA_HOST=http://localhost:11434"
Environment="LOG_LEVEL=INFO"
ExecStart=/opt/meetily/venv/bin/python main.py
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=3
StandardOutput=journal
StandardError=journal
SyslogIdentifier=meetily-backend

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/meetily/data /opt/meetily/logs

# Resource limits
LimitNOFILE=65536
MemoryMax=2G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
```

#### 2. Whisper Server Service
```ini
# /etc/systemd/system/meetily-whisper.service
[Unit]
Description=Meetily Whisper Server
After=network-online.target
Requires=network-online.target

[Service]
Type=simple
User=meetily
Group=meetily
WorkingDirectory=/opt/meetily/whisper-server
Environment="PATH=/usr/local/bin:/usr/bin:/bin"
ExecStart=/opt/meetily/whisper-server/run-server.sh --model models/ggml-small.bin --host 0.0.0.0 --port 8178
Restart=always
RestartSec=3
StandardOutput=journal
StandardError=journal
SyslogIdentifier=meetily-whisper

# Resource limits
MemoryMax=4G
CPUQuota=400%

[Install]
WantedBy=multi-user.target
```

### Installation Script

```bash
# install-systemd-services.sh
#!/bin/bash

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}Installing Meetily systemd services...${NC}"

# Create meetily user
if ! id "meetily" &>/dev/null; then
    echo -e "${BLUE}Creating meetily user...${NC}"
    sudo useradd -r -s /bin/false -m -d /opt/meetily meetily
fi

# Create directories
echo -e "${BLUE}Creating directories...${NC}"
sudo mkdir -p /opt/meetily/{backend,whisper-server,data,logs,venv}
sudo chown -R meetily:meetily /opt/meetily

# Copy application files
echo -e "${BLUE}Copying application files...${NC}"
sudo cp -r backend/* /opt/meetily/backend/
sudo cp -r whisper-server-package/* /opt/meetily/whisper-server/

# Set up Python virtual environment
echo -e "${BLUE}Setting up Python environment...${NC}"
sudo -u meetily python3 -m venv /opt/meetily/venv
sudo -u meetily /opt/meetily/venv/bin/pip install -r /opt/meetily/backend/requirements.txt

# Install systemd services
echo -e "${BLUE}Installing systemd services...${NC}"
sudo cp systemd/meetily-backend.service /etc/systemd/system/
sudo cp systemd/meetily-whisper.service /etc/systemd/system/

# Reload systemd and enable services
sudo systemctl daemon-reload
sudo systemctl enable meetily-backend.service
sudo systemctl enable meetily-whisper.service

# Start services
echo -e "${BLUE}Starting services...${NC}"
sudo systemctl start meetily-backend.service
sudo systemctl start meetily-whisper.service

# Check status
echo -e "${GREEN}Service status:${NC}"
sudo systemctl status meetily-backend.service --no-pager
sudo systemctl status meetily-whisper.service --no-pager

echo -e "${GREEN}Installation complete!${NC}"
echo -e "${BLUE}Backend API: http://localhost:5167${NC}"
echo -e "${BLUE}Whisper Server: http://localhost:8178${NC}"
```

### Service Management

```bash
# Start services
sudo systemctl start meetily-backend
sudo systemctl start meetily-whisper

# Enable auto-start
sudo systemctl enable meetily-backend
sudo systemctl enable meetily-whisper

# Check status
sudo systemctl status meetily-backend
sudo systemctl status meetily-whisper

# View logs
sudo journalctl -u meetily-backend -f
sudo journalctl -u meetily-whisper -f

# Restart services
sudo systemctl restart meetily-backend
sudo systemctl restart meetily-whisper

# Stop services
sudo systemctl stop meetily-backend
sudo systemctl stop meetily-whisper
```

---

## 🌐 Reverse Proxy Setup

### Nginx Configuration

#### 1. Basic Nginx Setup
```nginx
# /etc/nginx/sites-available/meetily
server {
    listen 80;
    server_name meetily.yourdomain.com;
    
    # Redirect HTTP to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name meetily.yourdomain.com;
    
    # SSL Configuration
    ssl_certificate /etc/ssl/certs/meetily.crt;
    ssl_certificate_key /etc/ssl/private/meetily.key;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    
    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=63072000; includeSubDomains; preload";
    
    # Backend API
    location /api/ {
        proxy_pass http://localhost:5167/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts for long-running requests
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 300s;
    }
    
    # Whisper Server
    location /whisper/ {
        proxy_pass http://localhost:8178/;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # File upload settings
        client_max_body_size 100M;
        proxy_request_buffering off;
    }
    
    # Static files (if serving web interface)
    location / {
        root /opt/meetily/web;
        index index.html;
        try_files $uri $uri/ /index.html;
        
        # Cache static assets
        location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg)$ {
            expires 1y;
            add_header Cache-Control "public, immutable";
        }
    }
    
    # Health check endpoint
    location /health {
        access_log off;
        return 200 "healthy\n";
        add_header Content-Type text/plain;
    }
}
```

#### 2. Load Balancing Configuration
```nginx
# /etc/nginx/conf.d/meetily-upstream.conf
upstream meetily_backend {
    least_conn;
    server localhost:5167 max_fails=3 fail_timeout=30s;
    server localhost:5168 max_fails=3 fail_timeout=30s backup;
}

upstream whisper_backend {
    ip_hash;  # Sticky sessions for file uploads
    server localhost:8178 max_fails=3 fail_timeout=30s;
    server localhost:8179 max_fails=3 fail_timeout=30s backup;
}

# Rate limiting
limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
limit_req_zone $binary_remote_addr zone=whisper:10m rate=2r/s;

server {
    listen 443 ssl http2;
    server_name meetily.yourdomain.com;
    
    # ... SSL configuration ...
    
    location /api/ {
        limit_req zone=api burst=20 nodelay;
        proxy_pass http://meetily_backend/;
        # ... other proxy settings ...
    }
    
    location /whisper/ {
        limit_req zone=whisper burst=5 nodelay;
        proxy_pass http://whisper_backend/;
        # ... other proxy settings ...
    }
}
```

### Apache Configuration

#### 1. Basic Apache Setup
```apache
# /etc/apache2/sites-available/meetily.conf
<VirtualHost *:80>
    ServerName meetily.yourdomain.com
    Redirect permanent / https://meetily.yourdomain.com/
</VirtualHost>

<VirtualHost *:443>
    ServerName meetily.yourdomain.com
    
    # SSL Configuration
    SSLEngine on
    SSLCertificateFile /etc/ssl/certs/meetily.crt
    SSLCertificateKeyFile /etc/ssl/private/meetily.key
    SSLProtocol all -SSLv3 -TLSv1 -TLSv1.1
    SSLCipherSuite ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384
    
    # Security headers
    Header always set X-Frame-Options DENY
    Header always set X-Content-Type-Options nosniff
    Header always set X-XSS-Protection "1; mode=block"
    Header always set Strict-Transport-Security "max-age=63072000; includeSubDomains; preload"
    
    # Backend API
    ProxyPreserveHost On
    ProxyPass /api/ http://localhost:5167/
    ProxyPassReverse /api/ http://localhost:5167/
    
    # Whisper Server
    ProxyPass /whisper/ http://localhost:8178/
    ProxyPassReverse /whisper/ http://localhost:8178/
    
    # Static files
    DocumentRoot /opt/meetily/web
    
    # Logging
    ErrorLog ${APACHE_LOG_DIR}/meetily_error.log
    CustomLog ${APACHE_LOG_DIR}/meetily_access.log combined
</VirtualHost>
```

---

## 📊 Monitoring and Logging

### Logging Configuration

#### 1. Centralized Logging with rsyslog
```bash
# /etc/rsyslog.d/meetily.conf
# Meetily application logs
:programname, isequal, "meetily-backend" /var/log/meetily/backend.log
:programname, isequal, "meetily-whisper" /var/log/meetily/whisper.log
& stop

# Create log directory
sudo mkdir -p /var/log/meetily
sudo chown syslog:adm /var/log/meetily
```

#### 2. Log Rotation
```bash
# /etc/logrotate.d/meetily
/var/log/meetily/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 syslog adm
    postrotate
        systemctl reload rsyslog
    endscript
}

/opt/meetily/logs/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 meetily meetily
    postrotate
        systemctl reload meetily-backend
    endscript
}
```

### Monitoring Setup

#### 1. Health Check Script
```bash
# /opt/meetily/scripts/health-check.sh
#!/bin/bash

# Configuration
BACKEND_URL="http://localhost:5167"
WHISPER_URL="http://localhost:8178"
OLLAMA_URL="http://localhost:11434"
LOG_FILE="/var/log/meetily/health-check.log"

# Function to log with timestamp
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
}

# Check backend health
check_backend() {
    if curl -sf "$BACKEND_URL/get-meetings" > /dev/null; then
        log "Backend: OK"
        return 0
    else
        log "Backend: FAILED"
        return 1
    fi
}

# Check Whisper server
check_whisper() {
    if curl -sf "$WHISPER_URL/" > /dev/null; then
        log "Whisper: OK"
        return 0
    else
        log "Whisper: FAILED"
        return 1
    fi
}

# Check Ollama
check_ollama() {
    if curl -sf "$OLLAMA_URL/api/tags" > /dev/null; then
        log "Ollama: OK"
        return 0
    else
        log "Ollama: FAILED"
        return 1
    fi
}

# Main health check
main() {
    local failed=0
    
    check_backend || failed=$((failed + 1))
    check_whisper || failed=$((failed + 1))
    check_ollama || failed=$((failed + 1))
    
    if [ $failed -eq 0 ]; then
        log "All services healthy"
        exit 0
    else
        log "$failed services failed"
        exit 1
    fi
}

main "$@"
```

#### 2. Prometheus Monitoring
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'meetily-backend'
    static_configs:
      - targets: ['localhost:5167']
    metrics_path: '/metrics'
    scrape_interval: 30s

  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']

  - job_name: 'nginx'
    static_configs:
      - targets: ['localhost:9113']
```

#### 3. Grafana Dashboard
```json
{
  "dashboard": {
    "title": "Meetily Monitoring",
    "panels": [
      {
        "title": "Service Status",
        "type": "stat",
        "targets": [
          {
            "expr": "up{job=\"meetily-backend\"}",
            "legendFormat": "Backend"
          }
        ]
      },
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])",
            "legendFormat": "Requests/sec"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "95th percentile"
          }
        ]
      }
    ]
  }
}
```

---

## 💾 Backup and Recovery

### Backup Strategy

#### 1. Database Backup Script
```bash
# /opt/meetily/scripts/backup-database.sh
#!/bin/bash

BACKUP_DIR="/opt/meetily/backups"
DB_PATH="/opt/meetily/data/meeting_minutes.db"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/meetily_db_$DATE.sqlite"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Create database backup
sqlite3 "$DB_PATH" ".backup '$BACKUP_FILE'"

# Compress backup
gzip "$BACKUP_FILE"

# Keep only last 30 days of backups
find "$BACKUP_DIR" -name "meetily_db_*.sqlite.gz" -mtime +30 -delete

echo "Database backup completed: $BACKUP_FILE.gz"
```

#### 2. Full System Backup
```bash
# /opt/meetily/scripts/full-backup.sh
#!/bin/bash

BACKUP_ROOT="/backup/meetily"
DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR="$BACKUP_ROOT/$DATE"

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup application files
tar -czf "$BACKUP_DIR/application.tar.gz" -C /opt/meetily \
    --exclude=venv \
    --exclude=logs \
    --exclude=*.pyc \
    .

# Backup database
cp /opt/meetily/data/meeting_minutes.db "$BACKUP_DIR/"

# Backup configuration
cp -r /etc/systemd/system/meetily-*.service "$BACKUP_DIR/"
cp -r /etc/nginx/sites-available/meetily "$BACKUP_DIR/" 2>/dev/null || true

# Create backup manifest
cat > "$BACKUP_DIR/manifest.txt" << EOF
Backup Date: $(date)
Application Version: $(cat /opt/meetily/VERSION 2>/dev/null || echo "Unknown")
Database Size: $(du -h /opt/meetily/data/meeting_minutes.db | cut -f1)
Total Backup Size: $(du -sh "$BACKUP_DIR" | cut -f1)
EOF

echo "Full backup completed: $BACKUP_DIR"
```

### Recovery Procedures

#### 1. Database Recovery
```bash
# /opt/meetily/scripts/restore-database.sh
#!/bin/bash

BACKUP_FILE="$1"
DB_PATH="/opt/meetily/data/meeting_minutes.db"

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: $0 <backup_file.sqlite.gz>"
    exit 1
fi

# Stop services
sudo systemctl stop meetily-backend

# Backup current database
cp "$DB_PATH" "$DB_PATH.backup.$(date +%Y%m%d_%H%M%S)"

# Restore from backup
if [[ "$BACKUP_FILE" == *.gz ]]; then
    gunzip -c "$BACKUP_FILE" > "$DB_PATH"
else
    cp "$BACKUP_FILE" "$DB_PATH"
fi

# Fix permissions
chown meetily:meetily "$DB_PATH"

# Start services
sudo systemctl start meetily-backend

echo "Database restored from: $BACKUP_FILE"
```

#### 2. Full System Recovery
```bash
# /opt/meetily/scripts/full-restore.sh
#!/bin/bash

BACKUP_DIR="$1"

if [ -z "$BACKUP_DIR" ] || [ ! -d "$BACKUP_DIR" ]; then
    echo "Usage: $0 <backup_directory>"
    exit 1
fi

# Stop services
sudo systemctl stop meetily-backend meetily-whisper

# Restore application files
tar -xzf "$BACKUP_DIR/application.tar.gz" -C /opt/meetily/

# Restore database
cp "$BACKUP_DIR/meeting_minutes.db" /opt/meetily/data/

# Restore configuration
cp "$BACKUP_DIR"/meetily-*.service /etc/systemd/system/
sudo systemctl daemon-reload

# Fix permissions
sudo chown -R meetily:meetily /opt/meetily

# Start services
sudo systemctl start meetily-backend meetily-whisper

echo "Full system restored from: $BACKUP_DIR"
```

---

## 🔍 Troubleshooting

### Common Build Issues

#### 1. Rust Compilation Errors
```bash
# Clear Rust cache
cargo clean

# Update Rust toolchain
rustup update

# Check for missing dependencies
sudo apt install build-essential cmake pkg-config libssl-dev

# Verify Rust installation
rustc --version
cargo --version
```

#### 2. Node.js Build Issues
```bash
# Clear npm/pnpm cache
pnpm cache clean
rm -rf node_modules package-lock.json

# Reinstall dependencies
pnpm install

# Check Node.js version
node --version  # Should be 18+
```

#### 3. Python Environment Issues
```bash
# Recreate virtual environment
rm -rf venv
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# Check Python version
python3 --version  # Should be 3.8+
```

### Deployment Issues

#### 1. Service Start Failures
```bash
# Check service status
sudo systemctl status meetily-backend
sudo systemctl status meetily-whisper

# View detailed logs
sudo journalctl -u meetily-backend -f
sudo journalctl -u meetily-whisper -f

# Check configuration
sudo systemctl cat meetily-backend
```

#### 2. Permission Issues
```bash
# Fix file permissions
sudo chown -R meetily:meetily /opt/meetily
sudo chmod +x /opt/meetily/backend/main.py

# Check SELinux (if enabled)
sudo setsebool -P httpd_can_network_connect 1
```

#### 3. Network Issues
```bash
# Check port availability
netstat -tlnp | grep -E "(5167|8178|11434)"

# Test connectivity
curl -v http://localhost:5167/get-meetings
curl -v http://localhost:8178/
curl -v http://localhost:11434/api/tags

# Check firewall
sudo ufw status
sudo iptables -L
```

### Performance Issues

#### 1. High Memory Usage
```bash
# Monitor memory usage
htop
free -h
ps aux | grep -E "(meetily|ollama)"

# Optimize Ollama
export OLLAMA_MAX_LOADED_MODELS=1
sudo systemctl restart ollama
```

#### 2. Slow Response Times
```bash
# Check system load
uptime
iostat 1 5

# Monitor database performance
sqlite3 /opt/meetily/data/meeting_minutes.db ".timer on" "EXPLAIN QUERY PLAN SELECT * FROM meetings;"

# Check disk space
df -h
```

### Recovery Procedures

#### 1. Service Recovery
```bash
# Restart all services
sudo systemctl restart meetily-backend meetily-whisper ollama

# Reset to known good state
sudo systemctl stop meetily-backend
cp /opt/meetily/backups/latest/meeting_minutes.db /opt/meetily/data/
sudo systemctl start meetily-backend
```

#### 2. Emergency Procedures
```bash
# Kill all related processes
sudo pkill -f meetily
sudo pkill -f ollama

# Clean temporary files
rm -rf /tmp/meetily-*
rm -rf /opt/meetily/logs/*

# Restart from clean state
sudo systemctl start meetily-backend meetily-whisper
```

---

## 🎯 Deployment Checklist

### Pre-deployment Checklist

- [ ] All dependencies installed and verified
- [ ] Build completes successfully without errors
- [ ] All tests pass (when available)
- [ ] Configuration files reviewed and updated
- [ ] SSL certificates installed (for production)
- [ ] Firewall rules configured
- [ ] Backup procedures tested
- [ ] Monitoring configured
- [ ] Log rotation configured

### Post-deployment Checklist

- [ ] All services start successfully
- [ ] Health checks pass
- [ ] API endpoints respond correctly
- [ ] Frontend loads and functions
- [ ] Audio recording works
- [ ] Transcription works
- [ ] AI summarization works
- [ ] Database operations work
- [ ] Backup procedures work
- [ ] Monitoring alerts configured
- [ ] Documentation updated

---

## 🎉 Conclusion

You now have comprehensive build and deployment procedures for Meetily that cover:

- **Development Environment**: Quick setup for contributors
- **Production Builds**: Optimized builds for deployment
- **Docker Deployment**: Containerized deployment options
- **Systemd Services**: Native Linux service deployment
- **Reverse Proxy**: Professional web server configuration
- **Monitoring**: Health checks and performance monitoring
- **Backup/Recovery**: Data protection and disaster recovery

This guide ensures reliable, scalable deployment of Meetily in any environment from development to enterprise production.