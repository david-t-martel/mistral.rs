#!/bin/bash

# Setup script for mistral.rs monitoring stack

set -e

echo "🚀 Setting up mistral.rs monitoring stack..."

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if docker-compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "❌ docker-compose is not installed. Please install docker-compose first."
    exit 1
fi

# Create necessary directories
echo "📁 Creating directories..."
mkdir -p /var/log/mistralrs
mkdir -p ./grafana-provisioning/datasources
mkdir -p ./grafana-provisioning/dashboards

# Set proper permissions
echo "🔐 Setting permissions..."
chmod 755 /var/log/mistralrs

# Check if running with GPU support
if command -v nvidia-smi &> /dev/null; then
    echo "🎮 GPU detected, enabling GPU monitoring..."
    export COMPOSE_PROFILES="gpu,cache"
else
    echo "💻 No GPU detected, using CPU monitoring only..."
    export COMPOSE_PROFILES="cache"
fi

# Generate secrets if not exists
if [ ! -f .env ]; then
    echo "🔑 Generating secrets..."
    cat > .env << EOF
# Grafana admin password
GF_SECURITY_ADMIN_PASSWORD=$(openssl rand -base64 12)

# Alertmanager webhook token
WEBHOOK_TOKEN=$(openssl rand -hex 32)

# Prometheus remote write password (if using)
REMOTE_WRITE_PASSWORD=$(openssl rand -base64 12)
EOF
    echo "✅ Secrets generated in .env file"
fi

# Pull latest images
echo "📥 Pulling Docker images..."
docker-compose pull

# Start the monitoring stack
echo "🎬 Starting monitoring stack..."
docker-compose up -d

# Wait for services to be ready
echo "⏳ Waiting for services to be ready..."
sleep 10

# Check service health
echo "🏥 Checking service health..."

# Check Prometheus
if curl -s http://localhost:9090/-/healthy > /dev/null; then
    echo "✅ Prometheus is healthy"
else
    echo "❌ Prometheus is not responding"
fi

# Check Grafana
if curl -s http://localhost:3000/api/health > /dev/null; then
    echo "✅ Grafana is healthy"
else
    echo "❌ Grafana is not responding"
fi

# Check Loki
if curl -s http://localhost:3100/ready > /dev/null; then
    echo "✅ Loki is healthy"
else
    echo "❌ Loki is not responding"
fi

# Check Alertmanager
if curl -s http://localhost:9093/-/healthy > /dev/null; then
    echo "✅ Alertmanager is healthy"
else
    echo "❌ Alertmanager is not responding"
fi

# Import Grafana dashboards
echo "📊 Importing Grafana dashboards..."

# Get Grafana admin password from .env if it exists
# shellcheck disable=SC1091
if [ -f .env ]; then
    # shellcheck disable=SC1091
    . ./.env
fi

# Wait for Grafana to fully start
sleep 5

# Create API key
API_KEY=$(curl -s -X POST \
    -H "Content-Type: application/json" \
    -d '{"name":"setup-script","role":"Admin"}' \
    "http://admin:${GF_SECURITY_ADMIN_PASSWORD}@localhost:3000/api/auth/keys" \
    | jq -r .key)

if [ -n "$API_KEY" ]; then
    # Import overview dashboard
    curl -X POST \
        -H "Authorization: Bearer ${API_KEY}" \
        -H "Content-Type: application/json" \
        -d @grafana-dashboard-overview.json \
        http://localhost:3000/api/dashboards/db

    # Import performance dashboard
    curl -X POST \
        -H "Authorization: Bearer ${API_KEY}" \
        -H "Content-Type: application/json" \
        -d @grafana-dashboard-performance.json \
        http://localhost:3000/api/dashboards/db

    echo "✅ Dashboards imported successfully"
else
    echo "⚠️ Could not create API key. Please import dashboards manually."
fi

echo ""
echo "🎉 Monitoring stack setup complete!"
echo ""
echo "📍 Access points:"
echo "  • Prometheus: http://localhost:9090"
echo "  • Grafana: http://localhost:3000 (admin/${GF_SECURITY_ADMIN_PASSWORD})"
echo "  • Alertmanager: http://localhost:9093"
echo "  • Loki: http://localhost:3100"
echo ""
echo "📚 Next steps:"
echo "  1. Configure mistral.rs to expose metrics on port 9090"
echo "  2. Update prometheus.yml with your mistral.rs endpoints"
echo "  3. Configure alert notification channels in Alertmanager"
echo "  4. Customize dashboards in Grafana as needed"
echo ""
echo "🛑 To stop the monitoring stack: docker-compose down"
echo "💾 To stop and remove all data: docker-compose down -v"
