# mistral.rs Deployment Guide

Complete deployment infrastructure for production-grade mistral.rs server deployment.

## Quick Start

```bash
# Docker deployment (recommended)
docker-compose up -d

# Linux systemd service
sudo systemctl start mistralrs-server

# Windows service
.\install-service.ps1 -Install
.\install-service.ps1 -Start
```

## Table of Contents

- [Overview](#overview)
- [Deployment Methods](#deployment-methods)
- [Configuration](#configuration)
- [Health Checks](#health-checks)
- [Monitoring](#monitoring)
- [Rollback Procedures](#rollback-procedures)
- [Troubleshooting](#troubleshooting)

## Overview

### Architecture

```
┌─────────────────────────────────────────────────┐
│                Load Balancer                     │
│            (nginx / traefik / etc)              │
└─────────────────┬───────────────────────────────┘
                  │
         ┌────────┴────────┐
         │                 │
    ┌────▼────┐      ┌────▼────┐
    │ Server  │      │ Server  │
    │   #1    │      │   #2    │
    └────┬────┘      └────┬────┘
         │                │
         └────────┬───────┘
                  │
         ┌────────▼────────┐
         │  Redis (Cache)   │
         │  MCP Servers     │
         │  Monitoring      │
         └──────────────────┘
```

### Resource Requirements

| Component          | CPU      | RAM   | VRAM  | Disk   |
| ------------------ | -------- | ----- | ----- | ------ |
| Small model (1.5B) | 2 cores  | 2GB   | 2GB   | 10GB   |
| Medium model (7B)  | 4 cores  | 4GB   | 6GB   | 20GB   |
| Large model (70B)  | 8+ cores | 16GB+ | 40GB+ | 100GB+ |

### Supported Platforms

- Linux (Ubuntu 22.04+, Debian 12+, RHEL 8+)
- Windows Server 2019+
- Docker (with NVIDIA GPU support)
- Kubernetes (Helm charts provided)

## Deployment Methods

### 1. Docker Deployment (Recommended)

#### Prerequisites

- Docker 24.0+
- Docker Compose 2.20+
- NVIDIA Container Toolkit (for GPU support)

#### Setup

```bash
# Clone repository
git clone https://github.com/EricLBuehler/mistral.rs
cd mistral.rs

# Configure environment
cp configs/common/.env.example .env
# Edit .env with your settings

# Build image
docker build -t mistralrs:latest .

# Or use docker-compose
docker-compose up -d
```

#### GPU Support

```bash
# Install NVIDIA Container Toolkit
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
    sudo tee /etc/apt/sources.list.d/nvidia-docker.list

sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit
sudo systemctl restart docker

# Test GPU access
docker run --rm --gpus all nvidia/cuda:12.9.0-base-ubuntu24.04 nvidia-smi
```

#### Docker Compose Services

The `docker-compose.yml` includes:

- **mistralrs**: Main inference server
- **redis**: Cache and RAG backend
- **prometheus**: Metrics collection
- **grafana**: Visualization dashboard

#### Access Points

- API: http://localhost:8080
- Grafana: http://localhost:3000 (admin/admin)
- Prometheus: http://localhost:9090
- Redis: localhost:6379

### 2. Linux Systemd Service

#### Prerequisites

- Rust 1.86+
- CUDA 12.9+ (for GPU)
- systemd-enabled Linux distribution

#### Installation

```bash
# Build binary
make build-cuda-full

# Create service user
sudo useradd -r -s /bin/false mistralrs

# Create directories
sudo mkdir -p /opt/mistralrs /var/lib/mistralrs/{models,cache} /etc/mistralrs /var/log/mistralrs
sudo chown -R mistralrs:mistralrs /opt/mistralrs /var/lib/mistralrs /var/log/mistralrs

# Copy binary
sudo cp target/release/mistralrs-server /opt/mistralrs/
sudo chown mistralrs:mistralrs /opt/mistralrs/mistralrs-server
sudo chmod +x /opt/mistralrs/mistralrs-server

# Copy chat templates
sudo cp -r chat_templates /opt/mistralrs/
sudo chown -R mistralrs:mistralrs /opt/mistralrs/chat_templates

# Install systemd service
sudo cp mistralrs-server.service /etc/systemd/system/
sudo systemctl daemon-reload

# Configure MCP servers
sudo cp configs/prod/mcp-config.json /etc/mistralrs/mcp-config.json
sudo chown mistralrs:mistralrs /etc/mistralrs/mcp-config.json

# Copy models (example with Qwen 1.5B)
sudo cp /path/to/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf /var/lib/mistralrs/models/
sudo chown mistralrs:mistralrs /var/lib/mistralrs/models/*.gguf

# Enable and start service
sudo systemctl enable mistralrs-server
sudo systemctl start mistralrs-server

# Check status
sudo systemctl status mistralrs-server
```

#### Service Management

```bash
# Start service
sudo systemctl start mistralrs-server

# Stop service
sudo systemctl stop mistralrs-server

# Restart service
sudo systemctl restart mistralrs-server

# View logs
sudo journalctl -u mistralrs-server -f

# Enable auto-start on boot
sudo systemctl enable mistralrs-server
```

### 3. Windows Service

#### Prerequisites

- Windows Server 2019+ or Windows 10/11
- Visual Studio 2022 Build Tools
- CUDA 12.9+ (for GPU)
- NSSM (Non-Sucking Service Manager)

#### Installation

```powershell
# Build binary
make build-cuda-full

# Install NSSM (via Chocolatey)
choco install nssm -y

# Run installer script as Administrator
.\install-service.ps1 -Install

# Start service
.\install-service.ps1 -Start

# Check status
.\install-service.ps1 -Status
```

#### Service Management

```powershell
# Start
.\install-service.ps1 -Start

# Stop
.\install-service.ps1 -Stop

# Restart
.\install-service.ps1 -Restart

# View logs
Get-Content .\logs\mistralrs-stderr.log -Tail 50 -Wait

# Uninstall
.\install-service.ps1 -Uninstall
```

### 4. Kubernetes Deployment

#### Prerequisites

- Kubernetes 1.28+
- Helm 3.12+
- NVIDIA GPU Operator (for GPU support)

#### Deploy with Helm

```bash
# Add Helm repository (example)
helm repo add mistralrs https://charts.mistralrs.dev
helm repo update

# Install with default values
helm install mistralrs mistralrs/mistralrs-server \
    --namespace llm \
    --create-namespace

# Install with custom values
helm install mistralrs mistralrs/mistralrs-server \
    --namespace llm \
    --create-namespace \
    --set image.tag=latest \
    --set resources.limits.nvidia.com/gpu=1 \
    --set model.path=/models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
```

#### Manual Kubernetes Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mistralrs-server
spec:
  replicas: 2
  selector:
    matchLabels:
      app: mistralrs
  template:
    metadata:
      labels:
        app: mistralrs
    spec:
      containers:
      - name: mistralrs
        image: mistralrs:latest
        ports:
        - containerPort: 8080
        resources:
          limits:
            nvidia.com/gpu: 1
            memory: 8Gi
          requests:
            cpu: 2
            memory: 4Gi
        volumeMounts:
        - name: models
          mountPath: /models
        - name: config
          mountPath: /config
        env:
        - name: RUST_LOG
          value: "info"
        - name: CUDA_VISIBLE_DEVICES
          value: "0"
      volumes:
      - name: models
        persistentVolumeClaim:
          claimName: mistralrs-models
      - name: config
        configMap:
          name: mistralrs-config
---
apiVersion: v1
kind: Service
metadata:
  name: mistralrs-service
spec:
  selector:
    app: mistralrs
  ports:
  - port: 8080
    targetPort: 8080
  type: LoadBalancer
```

```bash
kubectl apply -f deployment.yaml
```

## Configuration

### Environment Variables

See `configs/common/.env.example` for all available environment variables.

Key variables:

```bash
# Server
MISTRALRS_PORT=8080
MISTRALRS_HOST=0.0.0.0
MISTRALRS_MAX_SEQS=256

# Model
MODEL_DIR=/models
MODEL_FILE=Qwen2.5-1.5B-Instruct-Q4_K_M.gguf

# GPU
CUDA_VISIBLE_DEVICES=0

# Logging
RUST_LOG=info
```

### MCP Configuration

Environment-specific MCP configs:

- `configs/dev/mcp-config.json` - Development (minimal servers)
- `configs/staging/mcp-config.json` - Staging (moderate servers)
- `configs/prod/mcp-config.json` - Production (full servers)

### Model Management

#### Downloading Models

```bash
# Using Hugging Face CLI
huggingface-cli download TheBloke/Qwen2.5-1.5B-Instruct-GGUF \
    Qwen2.5-1.5B-Instruct-Q4_K_M.gguf \
    --local-dir /models

# Or manually
wget https://huggingface.co/TheBloke/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf \
    -O /models/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
```

#### Model Inventory

See `docs/MODEL_INVENTORY.json` for tested models and their specifications.

## Health Checks

### Automated Health Check Scripts

```bash
# Linux
./health-check.sh

# Windows
.\health-check.ps1

# Verbose mode
./health-check.sh --verbose
.\health-check.ps1 -Verbose
```

### Manual Health Checks

```bash
# HTTP endpoint
curl http://localhost:8080/health

# API test
curl -X POST http://localhost:8080/v1/completions \
    -H "Content-Type: application/json" \
    -d '{"prompt": "Hello", "max_tokens": 10}'

# Check GPU usage
nvidia-smi

# Check process
ps aux | grep mistralrs-server
```

### Health Check Endpoints

| Endpoint     | Purpose            | Expected Response    |
| ------------ | ------------------ | -------------------- |
| `/health`    | Basic health       | 200 OK               |
| `/v1/models` | List models        | JSON with model info |
| `/metrics`   | Prometheus metrics | Text format metrics  |

## Monitoring

### Prometheus Metrics

Metrics available at `http://localhost:8080/metrics`:

- Request latency
- Request count
- Model inference time
- GPU memory usage
- Cache hit rate

### Grafana Dashboards

Pre-configured dashboards:

1. **Overview Dashboard**: High-level metrics
1. **Performance Dashboard**: Detailed performance metrics
1. **Resource Dashboard**: CPU, memory, GPU usage

Access: http://localhost:3000 (default: admin/admin)

### Logging

#### Docker Logs

```bash
docker-compose logs -f mistralrs
```

#### Systemd Logs

```bash
sudo journalctl -u mistralrs-server -f
```

#### Windows Logs

```powershell
Get-Content .\logs\mistralrs-stderr.log -Tail 50 -Wait
```

## Rollback Procedures

### Docker Rollback

```bash
# Stop current deployment
docker-compose down

# Revert to previous image
docker pull mistralrs:v0.5.0  # Previous version
docker tag mistralrs:v0.5.0 mistralrs:latest

# Restart
docker-compose up -d
```

### Systemd Rollback

```bash
# Stop service
sudo systemctl stop mistralrs-server

# Restore previous binary
sudo cp /opt/mistralrs/mistralrs-server.backup /opt/mistralrs/mistralrs-server

# Start service
sudo systemctl start mistralrs-server
```

### Windows Rollback

```powershell
# Stop service
.\install-service.ps1 -Stop

# Restore previous binary
Copy-Item .\target\release\mistralrs-server.exe.backup .\target\release\mistralrs-server.exe -Force

# Start service
.\install-service.ps1 -Start
```

## Troubleshooting

### Common Issues

#### 1. Service Won't Start

**Check logs:**

```bash
# Docker
docker-compose logs mistralrs

# Linux
sudo journalctl -u mistralrs-server -n 50

# Windows
Get-Content .\logs\mistralrs-stderr.log -Tail 50
```

**Common causes:**

- Model file not found
- Insufficient VRAM
- Port already in use
- MCP server configuration error

#### 2. High Memory Usage

**Check memory:**

```bash
# Docker
docker stats

# Linux/Windows
nvidia-smi
```

**Solutions:**

- Use smaller model (1.5B instead of 7B)
- Reduce `MISTRALRS_MAX_SEQS`
- Enable model quantization

#### 3. Slow Inference

**Check GPU utilization:**

```bash
nvidia-smi dmon
```

**Solutions:**

- Ensure CUDA is properly configured
- Check `CUDA_VISIBLE_DEVICES`
- Verify model is loaded on GPU (not CPU)
- Increase `MISTRALRS_WORKERS`

#### 4. MCP Servers Not Working

**Check MCP processes:**

```bash
# Linux
ps aux | grep mcp

# Windows
Get-Process | Where-Object {$_.Name -like "*mcp*"}
```

**Solutions:**

- Verify npx is in PATH
- Check MCP config JSON syntax
- Ensure required npm packages are available

### Debug Mode

Enable verbose logging:

```bash
# Environment variable
RUST_LOG=debug MISTRALRS_DEBUG=1 ./mistralrs-server ...

# Or in .env
RUST_LOG=debug
MISTRALRS_DEBUG=1
```

### Performance Profiling

```bash
# Build with profiling
make build-profiled

# Run with perf (Linux)
perf record -g ./target/release/mistralrs-server ...
perf report

# Check VRAM usage
nvidia-smi dmon -s um
```

## Security Best Practices

1. **Run as non-root user** (systemd/Docker)
1. **Use secrets management** for API keys (not environment variables)
1. **Enable TLS** for production deployments
1. **Restrict file system access** (Docker volumes, systemd ProtectSystem)
1. **Regular security updates** (base images, dependencies)
1. **Network isolation** (Docker networks, firewalls)
1. **Audit logging** enabled

## Production Checklist

- [ ] Binary built with optimizations (`make build-cuda-full`)
- [ ] Models downloaded and validated
- [ ] Configuration files reviewed (especially MCP)
- [ ] Environment variables set correctly
- [ ] Health checks passing
- [ ] Monitoring configured (Prometheus + Grafana)
- [ ] Logging configured and working
- [ ] Backup strategy defined
- [ ] Rollback plan tested
- [ ] Load testing completed
- [ ] Security audit passed
- [ ] Documentation updated

## Support

- GitHub Issues: https://github.com/EricLBuehler/mistral.rs/issues
- Discord: https://discord.gg/SZrecqK8qw
- Documentation: https://ericlbuehler.github.io/mistral.rs/

## License

MIT License - See LICENSE file for details
