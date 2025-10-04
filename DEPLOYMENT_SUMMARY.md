# Deployment Infrastructure Summary

Complete production-ready deployment infrastructure for mistral.rs LLM inference server.

## Created Files

### Core Deployment

1. **Dockerfile** (4.4 KB)

   - Multi-stage build with CUDA 12.9 support
   - Optimized dependency caching
   - Non-root user for security
   - Health checks built-in
   - Production-ready runtime

1. **docker-compose.yml** (4.6 KB)

   - Full stack: Server + Redis + Prometheus + Grafana
   - GPU support with NVIDIA runtime
   - Volume mounts for models, configs, logs
   - Health checks for all services
   - Network isolation

1. **mistralrs-server.service** (1.3 KB)

   - Systemd service file for Linux
   - Auto-restart on failure
   - Resource limits (8GB memory)
   - Security hardening (ProtectSystem, PrivateTmp)
   - Logging to systemd journal

1. **install-service.ps1** (7.6 KB)

   - Windows service installer using NSSM
   - Parameter validation
   - Log rotation (daily, 100MB)
   - Auto-start configuration
   - Status monitoring

### Health & Monitoring

5. **health-check.sh** (6.4 KB)

   - Comprehensive health validation
   - HTTP, API, MCP connectivity checks
   - Resource usage monitoring
   - GPU memory tracking
   - Exit codes for automation

1. **health-check.ps1** (6.8 KB)

   - Windows equivalent of health-check.sh
   - PowerShell-native implementation
   - Same checks as Linux version
   - Colored output

### Configuration Management

7. **configs/dev/mcp-config.json**

   - Minimal MCP servers for development
   - Memory, Filesystem, Time servers
   - 60s timeout, 2 concurrent calls

1. **configs/staging/mcp-config.json**

   - Moderate MCP configuration
   - Adds Fetch, Sequential Thinking
   - 120s timeout, 3 concurrent calls

1. **configs/prod/mcp-config.json**

   - Full production MCP stack
   - Includes GitHub integration
   - 180s timeout, 5 concurrent calls

1. **configs/common/.env.example**

   - Comprehensive environment variable template
   - Covers server, model, GPU, cache, security
   - Inline documentation

### Documentation & Scripts

11. **DEPLOYMENT.md** (15 KB)

    - Complete deployment guide
    - Docker, systemd, Windows, Kubernetes
    - Configuration, monitoring, troubleshooting
    - Production checklist

01. **deploy.sh** (7.1 KB)

    - Automated deployment script
    - Supports docker, systemd, manual methods
    - Environment selection (dev/staging/prod)
    - Build integration

### Supporting Files

13. **prometheus.yml** (2.6 KB)
    - Metrics collection configuration (from monitoring/)

## Quick Start

### Docker (Recommended)

```bash
# Copy and configure environment
cp configs/common/.env.example .env
# Edit .env with your settings

# Deploy entire stack
docker-compose up -d

# Check status
./health-check.sh

# Access points
# API: http://localhost:8080
# Grafana: http://localhost:3000
```

### Linux Systemd

```bash
# Build binary
make build-cuda-full

# Deploy with script (requires sudo)
sudo ./deploy.sh --method systemd --environment prod

# Or manual deployment
sudo systemctl start mistralrs-server
```

### Windows Service

```powershell
# Build binary
make build-cuda-full

# Install service (as Administrator)
.\install-service.ps1 -Install

# Start service
.\install-service.ps1 -Start

# Check status
.\install-service.ps1 -Status
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│            Load Balancer (nginx/etc)            │
└──────────────────┬──────────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
   ┌────▼────┐          ┌────▼────┐
   │ Server  │          │ Server  │
   │  (GPU)  │          │  (GPU)  │
   └────┬────┘          └────┬────┘
        │                    │
        └──────────┬─────────┘
                   │
        ┌──────────▼──────────┐
        │   Redis (Cache)      │
        │   MCP Servers        │
        │   Prometheus/Grafana │
        └──────────────────────┘
```

## Resource Requirements

### Small Model (Qwen 1.5B Q4)

- CPU: 2 cores
- RAM: 2GB system
- VRAM: 1.2GB
- Disk: 10GB

### Medium Model (Qwen 7B Q4)

- CPU: 4 cores
- RAM: 4GB system
- VRAM: 5.5GB
- Disk: 20GB

### Large Model (70B Q4)

- CPU: 8+ cores
- RAM: 16GB+ system
- VRAM: 40GB+
- Disk: 100GB+

## Security Features

### Docker

- Non-root user execution
- Read-only volumes for models/configs
- Network isolation
- Resource limits enforced

### Systemd

- Dedicated service user (mistralrs)
- ProtectSystem=strict
- PrivateTmp=true
- RestrictNamespaces=true
- File system restrictions

### Windows

- Service isolation
- Log rotation
- Resource monitoring
- Controlled restart behavior

## Configuration Hierarchy

```
configs/
├── common/
│   └── .env.example         # Base environment template
├── dev/
│   └── mcp-config.json      # Development (minimal)
├── staging/
│   └── mcp-config.json      # Staging (moderate)
└── prod/
    └── mcp-config.json      # Production (full)
```

## Health Check Matrix

| Check             | Linux | Windows | Docker | Exit Code |
| ----------------- | ----- | ------- | ------ | --------- |
| HTTP connectivity | ✓     | ✓       | ✓      | 1         |
| /health endpoint  | ✓     | ✓       | ✓      | 2         |
| API endpoints     | ✓     | ✓       | ✓      | -         |
| MCP connectivity  | ✓     | ✓       | ✓      | 3         |
| Model loaded      | ✓     | ✓       | ✓      | -         |
| Resource usage    | ✓     | ✓       | ✓      | 4         |
| Disk space        | ✓     | ✓       | ✓      | -         |

## Monitoring Stack

### Prometheus

- Scrapes `/metrics` endpoint every 15s
- Retains 30 days of data
- Available: http://localhost:9090

### Grafana

- Pre-configured datasource
- Overview + Performance dashboards
- Available: http://localhost:3000
- Default: admin/admin

### Logs

- Docker: `docker-compose logs -f`
- Systemd: `journalctl -u mistralrs-server -f`
- Windows: `.\logs\mistralrs-stderr.log`

## Deployment Checklist

- [ ] Binary compiled with CUDA support
- [ ] Models downloaded to correct directory
- [ ] Configuration files reviewed
- [ ] Environment variables set
- [ ] Health checks passing
- [ ] Monitoring configured
- [ ] Logs accessible
- [ ] Backup strategy defined
- [ ] Rollback plan tested
- [ ] Load testing completed
- [ ] Security audit passed

## Rollback Strategy

### Docker

```bash
docker-compose down
docker tag mistralrs:v0.5.0 mistralrs:latest
docker-compose up -d
```

### Systemd

```bash
sudo systemctl stop mistralrs-server
sudo cp /opt/mistralrs/mistralrs-server.backup /opt/mistralrs/mistralrs-server
sudo systemctl start mistralrs-server
```

### Windows

```powershell
.\install-service.ps1 -Stop
Copy-Item backup\mistralrs-server.exe target\release\ -Force
.\install-service.ps1 -Start
```

## Testing Commands

```bash
# Quick health check
./health-check.sh

# API test
curl -X POST http://localhost:8080/v1/completions \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello", "max_tokens": 10}'

# Load test (with Apache Bench)
ab -n 100 -c 10 http://localhost:8080/health

# GPU monitoring
watch -n 1 nvidia-smi
```

## Performance Tuning

### Key Parameters

```bash
# Environment variables
MISTRALRS_WORKERS=2          # Number of worker threads (2x CPU cores)
MISTRALRS_MAX_SEQS=256       # Max concurrent sequences
CUDA_VISIBLE_DEVICES=0       # GPU selection

# Resource limits (docker-compose.yml)
mem_limit: 8g                # System memory limit
memswap_limit: 16g          # Swap limit

# Systemd limits
MemoryLimit=8G
TasksMax=512
```

## Troubleshooting

### Service won't start

1. Check logs (docker logs / journalctl / Windows logs)
1. Verify model file exists and is readable
1. Check port 8080 is not in use
1. Verify CUDA is available (nvidia-smi)

### High memory usage

1. Use smaller model (1.5B instead of 7B)
1. Reduce MISTRALRS_MAX_SEQS
1. Enable model quantization

### Slow inference

1. Verify GPU is being used (nvidia-smi)
1. Check CUDA_VISIBLE_DEVICES is set correctly
1. Increase MISTRALRS_WORKERS
1. Check network latency if using remote clients

## Production Best Practices

1. **Zero-downtime deployment**: Use blue-green or rolling updates
1. **Monitoring**: Alert on high latency, errors, resource usage
1. **Backups**: Regular backups of models and configurations
1. **Security**: TLS, authentication, rate limiting
1. **Scaling**: Horizontal scaling with load balancer
1. **Testing**: Load testing before production deployment

## File Locations

### Linux

- Binary: `/opt/mistralrs/mistralrs-server`
- Models: `/var/lib/mistralrs/models/`
- Config: `/etc/mistralrs/mcp-config.json`
- Logs: `/var/log/mistralrs/` or `journalctl`
- Cache: `/var/lib/mistralrs/cache/`

### Windows

- Binary: `target\release\mistralrs-server.exe`
- Models: `C:\codedev\llm\.models\`
- Config: `configs\prod\mcp-config.json`
- Logs: `logs\mistralrs-stderr.log`
- Cache: `%USERPROFILE%\.cache\huggingface\`

### Docker

- Binary: `/app/mistralrs-server`
- Models: `/models` (mounted)
- Config: `/config` (mounted)
- Logs: `/logs` (mounted)
- Cache: `/data` (mounted)

## Support & Resources

- Documentation: [DEPLOYMENT.md](DEPLOYMENT.md)
- GitHub: https://github.com/EricLBuehler/mistral.rs
- Discord: https://discord.gg/SZrecqK8qw
- Issues: https://github.com/EricLBuehler/mistral.rs/issues

## Version Information

- mistral.rs: v0.6.0
- Rust: 1.86+
- CUDA: 12.9
- Docker: 24.0+
- Docker Compose: 2.20+

## License

MIT License - See LICENSE file for details
