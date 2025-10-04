# PowerShell setup script for mistral.rs monitoring stack on Windows

Write-Host "🚀 Setting up mistral.rs monitoring stack..." -ForegroundColor Green

# Check if Docker is installed
try {
    docker --version | Out-Null
} catch {
    Write-Host "❌ Docker is not installed. Please install Docker Desktop for Windows first." -ForegroundColor Red
    exit 1
}

# Check if docker-compose is installed
try {
    docker-compose --version | Out-Null
} catch {
    Write-Host "❌ docker-compose is not installed. Please install docker-compose first." -ForegroundColor Red
    exit 1
}

# Create necessary directories
Write-Host "📁 Creating directories..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path ".\logs\mistralrs" | Out-Null
New-Item -ItemType Directory -Force -Path ".\grafana-provisioning\datasources" | Out-Null
New-Item -ItemType Directory -Force -Path ".\grafana-provisioning\dashboards" | Out-Null

# Check if running with GPU support (NVIDIA)
$gpuSupport = $false
try {
    nvidia-smi | Out-Null
    Write-Host "🎮 GPU detected, enabling GPU monitoring..." -ForegroundColor Green
    $env:COMPOSE_PROFILES = "gpu,cache"
    $gpuSupport = $true
} catch {
    Write-Host "💻 No GPU detected, using CPU monitoring only..." -ForegroundColor Yellow
    $env:COMPOSE_PROFILES = "cache"
}

# Generate secrets if not exists
if (-not (Test-Path .env)) {
    Write-Host "🔑 Generating secrets..." -ForegroundColor Yellow

    # Generate random passwords
    $grafanaPassword = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 12 | % {[char]$_})
    $webhookToken = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | % {[char]$_})
    $remoteWritePassword = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 12 | % {[char]$_})

    # Write .env file
    @"
# Grafana admin password
GF_SECURITY_ADMIN_PASSWORD=$grafanaPassword

# Alertmanager webhook token
WEBHOOK_TOKEN=$webhookToken

# Prometheus remote write password (if using)
REMOTE_WRITE_PASSWORD=$remoteWritePassword
"@ | Out-File -FilePath .env -Encoding UTF8

    Write-Host "✅ Secrets generated in .env file" -ForegroundColor Green
}

# Load environment variables
Get-Content .env | ForEach-Object {
    if ($_ -match '^([^#].*)=(.*)$') {
        Set-Item -Path "env:$($matches[1])" -Value $matches[2]
    }
}

# Pull latest images
Write-Host "📥 Pulling Docker images..." -ForegroundColor Yellow
docker-compose pull

# Start the monitoring stack
Write-Host "🎬 Starting monitoring stack..." -ForegroundColor Yellow
docker-compose up -d

# Wait for services to be ready
Write-Host "⏳ Waiting for services to be ready..." -ForegroundColor Yellow
Start-Sleep -Seconds 15

# Check service health
Write-Host "🏥 Checking service health..." -ForegroundColor Yellow

# Check Prometheus
try {
    $response = Invoke-WebRequest -Uri "http://localhost:9090/-/healthy" -UseBasicParsing -ErrorAction Stop
    Write-Host "✅ Prometheus is healthy" -ForegroundColor Green
} catch {
    Write-Host "❌ Prometheus is not responding" -ForegroundColor Red
}

# Check Grafana
try {
    $response = Invoke-WebRequest -Uri "http://localhost:3000/api/health" -UseBasicParsing -ErrorAction Stop
    Write-Host "✅ Grafana is healthy" -ForegroundColor Green
} catch {
    Write-Host "❌ Grafana is not responding" -ForegroundColor Red
}

# Check Loki
try {
    $response = Invoke-WebRequest -Uri "http://localhost:3100/ready" -UseBasicParsing -ErrorAction Stop
    Write-Host "✅ Loki is healthy" -ForegroundColor Green
} catch {
    Write-Host "❌ Loki is not responding" -ForegroundColor Red
}

# Check Alertmanager
try {
    $response = Invoke-WebRequest -Uri "http://localhost:9093/-/healthy" -UseBasicParsing -ErrorAction Stop
    Write-Host "✅ Alertmanager is healthy" -ForegroundColor Green
} catch {
    Write-Host "❌ Alertmanager is not responding" -ForegroundColor Red
}

# Import Grafana dashboards
Write-Host "📊 Importing Grafana dashboards..." -ForegroundColor Yellow

# Wait for Grafana to fully start
Start-Sleep -Seconds 5

# Create API key
$grafanaAuth = "admin:$($env:GF_SECURITY_ADMIN_PASSWORD)"
$encodedAuth = [Convert]::ToBase64String([Text.Encoding]::ASCII.GetBytes($grafanaAuth))

try {
    $headers = @{
        "Authorization" = "Basic $encodedAuth"
        "Content-Type" = "application/json"
    }

    $body = @{
        name = "setup-script"
        role = "Admin"
    } | ConvertTo-Json

    $response = Invoke-RestMethod -Uri "http://localhost:3000/api/auth/keys" `
        -Method Post -Headers $headers -Body $body

    $apiKey = $response.key

    if ($apiKey) {
        # Import overview dashboard
        $dashboardContent = Get-Content -Path "grafana-dashboard-overview.json" -Raw
        $headers = @{
            "Authorization" = "Bearer $apiKey"
            "Content-Type" = "application/json"
        }

        Invoke-RestMethod -Uri "http://localhost:3000/api/dashboards/db" `
            -Method Post -Headers $headers -Body $dashboardContent

        # Import performance dashboard
        $dashboardContent = Get-Content -Path "grafana-dashboard-performance.json" -Raw
        Invoke-RestMethod -Uri "http://localhost:3000/api/dashboards/db" `
            -Method Post -Headers $headers -Body $dashboardContent

        Write-Host "✅ Dashboards imported successfully" -ForegroundColor Green
    }
} catch {
    Write-Host "⚠️ Could not import dashboards automatically. Please import manually." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "🎉 Monitoring stack setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "📍 Access points:" -ForegroundColor Cyan
Write-Host "  • Prometheus: http://localhost:9090"
Write-Host "  • Grafana: http://localhost:3000 (admin/$($env:GF_SECURITY_ADMIN_PASSWORD))"
Write-Host "  • Alertmanager: http://localhost:9093"
Write-Host "  • Loki: http://localhost:3100"
Write-Host ""
Write-Host "📚 Next steps:" -ForegroundColor Cyan
Write-Host "  1. Configure mistral.rs to expose metrics on port 9090"
Write-Host "  2. Update prometheus.yml with your mistral.rs endpoints"
Write-Host "  3. Configure alert notification channels in Alertmanager"
Write-Host "  4. Customize dashboards in Grafana as needed"
Write-Host ""
Write-Host "🛑 To stop the monitoring stack: docker-compose down" -ForegroundColor Yellow
Write-Host "💾 To stop and remove all data: docker-compose down -v" -ForegroundColor Yellow

if ($gpuSupport) {
    Write-Host ""
    Write-Host "🎮 GPU Monitoring enabled. Access GPU metrics at:" -ForegroundColor Green
    Write-Host "  • http://localhost:9835/metrics (NVIDIA GPU Exporter)"
}
