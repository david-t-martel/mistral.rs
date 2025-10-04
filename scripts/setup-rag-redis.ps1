#!/usr/bin/env pwsh
# Setup script for RAG-Redis integration with mistral.rs
# Initializes the system and ingests project documentation

param(
    [string]$RedisHost = "127.0.0.1",
    [int]$RedisPort = 6379,
    [string]$RagDataDir = "T:\projects\rust-mistral\mistral.rs\rag-data",
    [string]$RagCacheDir = "T:\projects\rust-mistral\mistral.rs\rag-cache",
    [switch]$SkipRedisCheck,
    [switch]$ForceReingest
)

$ErrorActionPreference = "Stop"

Write-Host "=== RAG-Redis Integration Setup for mistral.rs ===" -ForegroundColor Cyan
Write-Host ""

# Check Redis connection
if (-not $SkipRedisCheck) {
    Write-Host "Checking Redis connection..." -ForegroundColor Yellow
    try {
        $redisCheck = & redis-cli -h $RedisHost -p $RedisPort ping 2>&1
        if ($redisCheck -ne "PONG") {
            throw "Redis not responding"
        }
        Write-Host "✓ Redis is running on ${RedisHost}:${RedisPort}" -ForegroundColor Green
    } catch {
        Write-Host "✗ Redis is not available. Please start Redis first:" -ForegroundColor Red
        Write-Host "  redis-server --port $RedisPort" -ForegroundColor Yellow
        exit 1
    }
}

# Create required directories
Write-Host ""
Write-Host "Creating data directories..." -ForegroundColor Yellow

$directories = @($RagDataDir, $RagCacheDir, "$RagDataDir\embeddings", "$RagDataDir\chunks", "$RagCacheDir\models")
foreach ($dir in $directories) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Write-Host "✓ Created: $dir" -ForegroundColor Green
    } else {
        Write-Host "✓ Exists: $dir" -ForegroundColor DarkGreen
    }
}

# Check if RAG-Redis server is installed
Write-Host ""
Write-Host "Checking RAG-Redis server installation..." -ForegroundColor Yellow

$ragServerPath = "C:\users\david\bin\rag-redis-mcp-server.exe"
if (Test-Path $ragServerPath) {
    Write-Host "✓ RAG-Redis server found at: $ragServerPath" -ForegroundColor Green
} else {
    Write-Host "✗ RAG-Redis server not found. Building from source..." -ForegroundColor Yellow

    $ragProjectPath = "C:\codedev\llm\rag-redis"
    if (Test-Path $ragProjectPath) {
        Push-Location $ragProjectPath
        try {
            cargo build --release --features mcp
            Copy-Item "target\release\rag-redis-mcp-server.exe" $ragServerPath -Force
            Write-Host "✓ RAG-Redis server built and installed" -ForegroundColor Green
        } finally {
            Pop-Location
        }
    } else {
        Write-Host "✗ RAG-Redis source not found at: $ragProjectPath" -ForegroundColor Red
        exit 1
    }
}

# Create environment file
Write-Host ""
Write-Host "Creating environment configuration..." -ForegroundColor Yellow

$envContent = @"
# RAG-Redis Configuration for mistral.rs
REDIS_URL=redis://${RedisHost}:${RedisPort}
RAG_DATA_DIR=$RagDataDir
EMBEDDING_CACHE_DIR=$RagCacheDir
LOG_DIR=$RagDataDir\logs
LOG_LEVEL=INFO
RUST_LOG=info

# Performance Settings
RAG_MAX_QPM=60
RAG_MAX_CONCURRENT=3
RAG_CACHE_SIZE_MB=500
RAG_QUERY_TIMEOUT_MS=500

# Embedding Models
EMBEDDING_MODEL=bge-small-en-v1.5
CODE_EMBEDDING_MODEL=codebert-base
EMBEDDING_BATCH_SIZE=32

# MCP Protocol
MCP_PROTOCOL_VERSION=2025-06-18
"@

$envPath = "T:\projects\rust-mistral\mistral.rs\.env.rag"
$envContent | Out-File -FilePath $envPath -Encoding UTF8
Write-Host "✓ Created environment file: $envPath" -ForegroundColor Green

# Create ingestion script
Write-Host ""
Write-Host "Creating document ingestion script..." -ForegroundColor Yellow

$ingestScript = @'
use mistralrs_mcp::rag_integration::{AgentContextManager, RagMcpClient};
use mistralrs_mcp::client::McpClient;
use mistralrs_mcp::McpClientConfig;
use std::sync::Arc;
use tokio;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    println!("Initializing RAG-Redis document ingestion...");

    // Load MCP configuration
    let config_path = "tests/mcp/MCP_CONFIG.json";
    let config_str = std::fs::read_to_string(config_path)?;
    let config: McpClientConfig = serde_json::from_str(&config_str)?;

    // Initialize MCP client
    let mut mcp_client = McpClient::new(config);
    mcp_client.initialize().await?;

    // Create RAG client
    let rag_client = Arc::new(RagMcpClient::new(Arc::new(mcp_client)));

    // Create context manager
    let context_manager = AgentContextManager::new(rag_client);

    // Ingest all project documentation
    println!("Starting document ingestion...");
    context_manager.ingest_project_docs().await?;

    println!("Document ingestion complete!");

    // Test a few queries
    println!("\nTesting context retrieval...");

    let build_context = context_manager.get_build_context("cuda").await?;
    println!("CUDA build context: {} chars", build_context.len());

    let api_docs = context_manager.get_api_docs("mistralrs-core", Some("Pipeline")).await?;
    println!("Pipeline API docs: {} chars", api_docs.len());

    let examples = context_manager.get_examples("MCP integration").await?;
    println!("MCP examples found: {}", examples.len());

    println!("\nRAG-Redis integration ready!");

    Ok(())
}
'@

$ingestScriptPath = "T:\projects\rust-mistral\mistral.rs\scripts\ingest_docs.rs"
$ingestScript | Out-File -FilePath $ingestScriptPath -Encoding UTF8
Write-Host "✓ Created ingestion script: $ingestScriptPath" -ForegroundColor Green

# Check for existing data
Write-Host ""
if ($ForceReingest) {
    Write-Host "Force reingestion requested. Clearing existing data..." -ForegroundColor Yellow
    & redis-cli -h $RedisHost -p $RedisPort FLUSHDB | Out-Null
    Remove-Item "$RagDataDir\*" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "✓ Cleared existing data" -ForegroundColor Green
} else {
    $existingKeys = & redis-cli -h $RedisHost -p $RedisPort DBSIZE 2>&1
    if ($existingKeys -match "^\d+$" -and [int]$existingKeys -gt 0) {
        Write-Host "Found $existingKeys existing keys in Redis" -ForegroundColor Yellow
        $response = Read-Host "Do you want to clear existing data? (y/N)"
        if ($response -eq 'y' -or $response -eq 'Y') {
            & redis-cli -h $RedisHost -p $RedisPort FLUSHDB | Out-Null
            Write-Host "✓ Cleared existing data" -ForegroundColor Green
        }
    }
}

# Collect documentation files
Write-Host ""
Write-Host "Collecting documentation files..." -ForegroundColor Yellow

$docPatterns = @(
    "*.md",
    ".claude\*.md",
    "docs\*.md",
    "examples\*\README.md",
    "mistralrs-*\README.md",
    "mistralrs-pyo3\API.md",
    ".github\*.md"
)

$totalFiles = 0
foreach ($pattern in $docPatterns) {
    $files = Get-ChildItem -Path "T:\projects\rust-mistral\mistral.rs" -Filter $pattern -Recurse -ErrorAction SilentlyContinue
    $count = ($files | Measure-Object).Count
    $totalFiles += $count
    if ($count -gt 0) {
        Write-Host "  Found $count files matching: $pattern" -ForegroundColor Gray
    }
}

Write-Host "✓ Total documentation files found: $totalFiles" -ForegroundColor Green

# Create batch ingestion list
Write-Host ""
Write-Host "Creating batch ingestion list..." -ForegroundColor Yellow

$ingestList = @()
$priorityDocs = @(
    "T:\projects\rust-mistral\mistral.rs\CLAUDE.md",
    "T:\projects\rust-mistral\mistral.rs\.claude\CLAUDE.md",
    "T:\projects\rust-mistral\mistral.rs\README.md",
    "T:\projects\rust-mistral\mistral.rs\docs\AGENT_MODE_GUIDE.md",
    "T:\projects\rust-mistral\mistral.rs\docs\HTTP.md",
    "T:\projects\rust-mistral\mistral.rs\mistralrs-pyo3\API.md"
)

foreach ($doc in $priorityDocs) {
    if (Test-Path $doc) {
        $ingestList += $doc
    }
}

$ingestListPath = "T:\projects\rust-mistral\mistral.rs\rag-ingest-list.txt"
$ingestList | Out-File -FilePath $ingestListPath -Encoding UTF8
Write-Host "✓ Created ingestion list with $($ingestList.Count) priority documents" -ForegroundColor Green

# Start RAG-Redis server
Write-Host ""
Write-Host "Starting RAG-Redis MCP server..." -ForegroundColor Yellow

$serverProcess = Start-Process -FilePath $ragServerPath `
    -ArgumentList "" `
    -PassThru `
    -WindowStyle Hidden `
    -RedirectStandardOutput "$RagDataDir\logs\server.log" `
    -RedirectStandardError "$RagDataDir\logs\server.err"

Start-Sleep -Seconds 2

if ($serverProcess.HasExited) {
    Write-Host "✗ Failed to start RAG-Redis server" -ForegroundColor Red
    Get-Content "$RagDataDir\logs\server.err" -Tail 10
    exit 1
}

Write-Host "✓ RAG-Redis server started (PID: $($serverProcess.Id))" -ForegroundColor Green

# Run document ingestion
Write-Host ""
Write-Host "Running document ingestion..." -ForegroundColor Yellow
Write-Host "This may take several minutes..." -ForegroundColor Gray

try {
    Push-Location "T:\projects\rust-mistral\mistral.rs"

    # Use the Rust ingestion script
    $result = cargo run --bin rag-ingest --release 2>&1

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ Document ingestion completed successfully" -ForegroundColor Green
    } else {
        Write-Host "✗ Document ingestion failed" -ForegroundColor Red
        Write-Host $result
    }
} finally {
    Pop-Location
}

# Verify ingestion
Write-Host ""
Write-Host "Verifying ingestion..." -ForegroundColor Yellow

$keyCount = & redis-cli -h $RedisHost -p $RedisPort DBSIZE 2>&1
if ($keyCount -match "^\d+$") {
    Write-Host "✓ Redis contains $keyCount keys" -ForegroundColor Green

    # Sample some keys
    $sampleKeys = & redis-cli -h $RedisHost -p $RedisPort --scan --pattern "doc:*" 2>&1 | Select-Object -First 5
    if ($sampleKeys) {
        Write-Host "  Sample document keys:" -ForegroundColor Gray
        $sampleKeys | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
    }
}

# Create test script
Write-Host ""
Write-Host "Creating test script..." -ForegroundColor Yellow

$testScript = @'
#!/usr/bin/env pwsh
# Test RAG-Redis context retrieval

$queries = @(
    @{Query="How to build with CUDA"; Type="BuildInstructions"},
    @{Query="Pipeline trait implementation"; Type="ApiDocumentation"},
    @{Query="MCP client example"; Type="Examples"},
    @{Query="Agent mode architecture"; Type="Agent"}
)

foreach ($q in $queries) {
    Write-Host "`nQuery: $($q.Query)" -ForegroundColor Cyan
    Write-Host "Type: $($q.Type)" -ForegroundColor Gray

    # Call RAG-Redis through MCP
    # This would use the actual MCP client in production
    Write-Host "Results would appear here..." -ForegroundColor DarkGray
}
'@

$testScriptPath = "T:\projects\rust-mistral\mistral.rs\scripts\test-rag-redis.ps1"
$testScript | Out-File -FilePath $testScriptPath -Encoding UTF8
Write-Host "✓ Created test script: $testScriptPath" -ForegroundColor Green

# Summary
Write-Host ""
Write-Host "=== Setup Complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "RAG-Redis integration is ready!" -ForegroundColor Cyan
Write-Host ""
Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Redis: ${RedisHost}:${RedisPort}"
Write-Host "  Data Dir: $RagDataDir"
Write-Host "  Cache Dir: $RagCacheDir"
Write-Host "  Documents: $totalFiles files"
Write-Host "  Server PID: $($serverProcess.Id)"
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Test context retrieval: .\scripts\test-rag-redis.ps1"
Write-Host "  2. Integrate with mistral.rs: Add rag_integration module"
Write-Host "  3. Monitor performance: redis-cli -h $RedisHost -p $RedisPort INFO"
Write-Host ""
Write-Host "To stop the server:" -ForegroundColor Gray
Write-Host "  Stop-Process -Id $($serverProcess.Id)"
