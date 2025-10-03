# Comprehensive Testing & Evaluation Plan for mistral.rs
## Project: mistral.rs CUDA Agent with MCP Integration

**Date Created**: 2025-10-03  
**Working Directory**: `T:\projects\rust-mistral\mistral.rs`  
**Total Phases**: 24 (0-8 with subphases)

---

## Overview

This plan systematically tests the mistral.rs server with comprehensive coverage of:
- **MCP Server Integration** (9 servers)
- **TUI Interactive Mode** (text, vision, MCP-enabled)
- **PyO3 Python Bindings** (build, import, inference)
- **Configuration Review** (Cargo, scripts, environment)
- **Performance Benchmarking** (HTTP server, VRAM, throughput)
- **End-to-End Integration** (multi-modal, tool-calling)

All test outputs will be captured in `.testlogs/` directory and consolidated into comprehensive reports.

---

## ✅ Phase 0: Workspace Bootstrap [COMPLETED]

**Status**: Complete  
**Artifacts**: `.testlogs/` directory created, helper functions defined

**What Was Done**:
- Created `.testlogs/` directory for all test outputs
- Verified PATH includes `C:\Users\david\.local\bin`
- Started transcript logging
- Defined `Invoke-Logged` helper function for consistent test execution

---

## Phase 1: Pre-flight Verification

**Objective**: Verify binaries, dependencies, and system readiness

### Tasks:
1. Binary verification via `test-mistralrs.ps1` or rebuild
2. GPU/CUDA check with `nvidia-smi`
3. Environment setup via `setup-dev-env.ps1`
4. MODEL_INVENTORY.json presence check
5. Redis connectivity test
6. Node/Bun/uv/Python version capture

### Commands:
```powershell
$logDir = Join-Path (Get-Location) '.testlogs'

# 1. Verify main binary
if (Test-Path .\test-mistralrs.ps1) {
  .\test-mistralrs.ps1 -Verbose 2>&1 | Tee-Object "$logDir\preflight-test-mistralrs.log"
}

# 2. GPU / CUDA
nvidia-smi 2>&1 | Tee-Object "$logDir\nvidia-smi.log"

# 3. Dev env vars
if (Test-Path .\setup-dev-env.ps1) {
  .\setup-dev-env.ps1 2>&1 | Tee-Object "$logDir\setup-dev-env.log"
}

# 4. Model inventory
$invPath = Join-Path (Get-Location) 'MODEL_INVENTORY.json'
if (Test-Path $invPath) {
  Write-Host "MODEL_INVENTORY.json found."
} else {
  Write-Warning "MODEL_INVENTORY.json missing; will generate in Phase 1.1"
}

# 5. Redis check
$redisPing = (redis-cli ping 2>$null)
if ($redisPing -ne 'PONG') {
  Write-Warning "Redis not responding; RAG-Redis MCP will be SKIPPED"
  # Try to start if service exists
  Get-Service -Name 'redis*' -ErrorAction SilentlyContinue | ForEach-Object {
    if ($_.Status -ne 'Running') { Start-Service $_.Name }
  }
}

# 6. Tool versions
node -v 2>&1 | Tee-Object "$logDir\node-version.log"
npx -v  2>&1 | Tee-Object "$logDir\npx-version.log"
bun -v  2>&1 | Tee-Object "$logDir\bun-version.log"
uv --version 2>&1 | Tee-Object "$logDir\uv-version.log"
python --version 2>&1 | Tee-Object "$logDir\python-version.log"
```

**Expected Outputs**:
- Logs in `.testlogs/preflight-*`
- Binary confirmation or rebuild trigger
- GPU info captured
- Dependency versions logged

---

## Phase 1.1: MODEL_INVENTORY.json Generation

**Objective**: Scan and catalog all available models

### Command:
```powershell
$modelsRoot = 'C:\codedev\llm\.models'
if (Test-Path $modelsRoot) {
  $files = Get-ChildItem -Path $modelsRoot -Recurse -File -Include *.gguf,*.safetensors -ErrorAction SilentlyContinue
  $entries = @()
  foreach ($f in $files) {
    $entries += [pscustomobject]@{
      name = $f.BaseName
      path = $f.FullName
      format = $f.Extension.TrimStart('.')
      size_bytes = $f.Length
      modified_utc = $f.LastWriteTimeUtc.ToString("o")
    }
  }
  $entries | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 MODEL_INVENTORY.json
  Write-Host "MODEL_INVENTORY.json written with $($entries.Count) entries."
}
```

**Outputs**: `MODEL_INVENTORY.json`

---

## Phase 2: MCP Server Testing (Existing Scripts)

**Objective**: Run existing test harnesses and consolidate results

### Commands:
```powershell
# Run both existing test scripts
if (Test-Path .\test-mcp-servers.ps1) {
  .\test-mcp-servers.ps1 -Verbose 2>&1 | Tee-Object "$logDir\test-mcp-servers.log"
}
if (Test-Path .\test-phase2-mcp-servers.ps1) {
  .\test-phase2-mcp-servers.ps1 2>&1 | Tee-Object "$logDir\test-phase2-mcp-servers.log"
}

# Consolidate results
$phase2 = if (Test-Path PHASE2_TEST_RESULTS.json) { Get-Content PHASE2_TEST_RESULTS.json -Raw | ConvertFrom-Json } else { @() }
$valid  = if (Test-Path mcp-validation-results.json) { Get-Content mcp-validation-results.json -Raw | ConvertFrom-Json } else { @() }
@{ timestamp = (Get-Date).ToString("o"); phase2=$phase2; validation=$valid } | ConvertTo-Json -Depth 6 | Out-File -Encoding utf8 MCP_TEST_RESULTS.json
```

**Expected Files**:
- `.testlogs/test-mcp-servers.log`
- `.testlogs/test-phase2-mcp-servers.log`
- `MCP_TEST_RESULTS.json`

---

## Phase 2.1: Manual MCP Server Tests (9 Servers)

**Objective**: Individually test each MCP server with timeouts and logs

### Servers to Test:
1. **Memory** (npx @modelcontextprotocol/server-memory@2025.8.4)
2. **Filesystem** (npx @modelcontextprotocol/server-filesystem@2025.8.21)
3. **Sequential Thinking** (npx @modelcontextprotocol/server-sequential-thinking@2025.7.1)
4. **GitHub** (npx @modelcontextprotocol/server-github@2025.4.8) - Requires GITHUB_PERSONAL_ACCESS_TOKEN
5. **Fetch** (npx @modelcontextprotocol/server-fetch@0.6.3)
6. **Time** (npx @theo.foobar/mcp-time) - New replacement
7. **Serena Claude** (uv run python T:/projects/mcp_servers/serena/scripts/mcp_server.py)
8. **Desktop Commander** (uv --directory C:/Users/david/.claude/python_fileops run python -m desktop_commander.mcp_server)
9. **RAG-Redis** (C:/users/david/bin/rag-redis-mcp-server.exe) - Requires Redis

### Testing Pattern (per server):
```powershell
# Example for Memory server
$result = Invoke-Logged -Name 'mcp-memory' -TimeoutSec 20 -Script {
  npx -y @modelcontextprotocol/server-memory@2025.8.4
}
$manualResults += $result
```

**Cleanup After Tests**:
```powershell
Get-Process node,bun,python,rag-redis-mcp-server -ErrorAction SilentlyContinue | Stop-Process -Force
```

**Outputs**: Individual logs in `.testlogs/mcp-*.log`

---

## Phase 3A: TUI Interactive Test (Text Model)

**Objective**: Test TUI with smallest GGUF text model

### Commands:
```powershell
$inv = Get-Content .\MODEL_INVENTORY.json -Raw | ConvertFrom-Json
$textModel = ($inv | Where-Object { $_.format -eq 'gguf' -and $_.name -match 'qwen|gemma|phi|mistral' } | Sort-Object size_bytes | Select-Object -First 1).path

$cmds = @(
  "Hello! Summarize the project in one sentence.",
  "\system Always be concise",
  "\temperature 0.7",
  "\topk 50",
  "\help",
  "\clear",
  "List three core features and exit next.",
  "\exit"
)

$exe = ".\target\release\mistralrs-server.exe"
$cmds | & $exe -i gguf -m (Split-Path $textModel -Parent) -f $textModel 2>&1 | Tee-Object -FilePath "TUI_TEST_LOG.txt"
```

**Test Validation**:
- Prompts echo correctly
- Commands work: `\system`, `\temperature`, `\topk`, `\help`, `\clear`, `\exit`
- Process exits cleanly

**Outputs**: `TUI_TEST_LOG.txt`

---

## Phase 3B: TUI Vision Model Test (If Available)

**Objective**: Test vision capabilities in TUI

### Commands:
```powershell
$visionModel = ($inv | Where-Object { $_.format -eq 'gguf' -and $_.name -match 'vision|llava|qwen-vl|phi-vision' } | Sort-Object size_bytes | Select-Object -First 1).path

if ($visionModel) {
  $vcmds = @(
    "Describe the image at: C:\path\to\sample.jpg",
    "\help",
    "\exit"
  )
  $vcmds | & $exe -i vision-plain -m $visionModel 2>&1 | Tee-Object -FilePath "TUI_TEST_LOG.txt" -Append
} else {
  Write-Host "No vision model found; skipping vision TUI test."
}
```

---

## Phase 3C: TUI with MCP Integration

**Objective**: Test TUI with MCP tools enabled

### Commands:
```powershell
$cmdsMcp = @(
  "\help",
  "What tools are available?",
  "Use the filesystem tool to list this repo root.",
  "Fetch https://example.com and report the HTTP status.",
  "\exit"
)

$cmdsMcp | & $exe -i --mcp-config MCP_CONFIG.json gguf -m (Split-Path $textModel -Parent) -f $textModel 2>&1 | Tee-Object -FilePath "TUI_TEST_LOG.txt" -Append
```

**Validation**:
- Tools discovered at startup
- Tool listing works
- MCP tool calls succeed

---

## Phase 3D: TUI Code Review

**Objective**: Review interactive_mode.rs implementation

### Tasks:
1. Read `T:\projects\rust-mistral\mistral.rs\mistralrs-server\src\interactive_mode.rs`
2. Confirm history file location: `%APPDATA%\mistral.rs\history.txt`
3. Document slash commands and multi-modal support
4. Identify TODOs and panics

**Output**: Section in COMPREHENSIVE_TEST_REPORT.md

---

## Phase 4A: PyO3 Bindings Check

**Objective**: Verify PyO3 artifacts and features

### Commands:
```powershell
$wheelDir = Join-Path (Get-Location) 'target\wheels'
$pyd = Get-ChildItem -Path .\target\release -Filter mistralrs*.pyd -ErrorAction SilentlyContinue
$pyo3Cargo = 'mistralrs-pyo3\Cargo.toml'

if (Test-Path $pyo3Cargo) {
  Get-Content $pyo3Cargo | Select-String 'features|cuda|cudnn|mkl|flash-attn'
}
```

**Output**: PYO3_STATUS_REPORT.md

---

## Phase 4B: Build PyO3 via Maturin

**Objective**: Build Python bindings with CUDA support

### Commands:
```powershell
uv pip install -U maturin

Set-Location .\mistralrs-pyo3
maturin build --release --features cuda,flash-attn,cudnn,mkl 2>&1 | Tee-Object "$logDir\pyo3-maturin-build.log"
# Or: maturin develop --release --features cuda,flash-attn,cudnn,mkl
Set-Location ..
```

**Expected**: Wheel in `target/wheels/`

---

## Phase 4C: PyO3 Import and Inference Test

**Objective**: Smoke test Python bindings

### Commands:
```powershell
python -c @"
try:
    import mistralrs as mrs
    print('mistralrs version:', getattr(mrs, '__version__', 'unknown'))
    print('symbols:', [s for s in dir(mrs) if not s.startswith('_')][:20])
except Exception as e:
    print('IMPORT-ERROR:', e)
"@
```

**Output**: Results in PYO3_STATUS_REPORT.md

---

## Phase 5: Cargo Configuration Review

**Objective**: Review build settings and optimization flags

### Files to Review:
- `Cargo.toml` (workspace)
- `mistralrs-server/Cargo.toml`
- `.cargo/config.toml`

### Verify:
- Features: `cuda`, `flash-attn`, `cudnn`, `mkl` wired correctly
- Release profile: `opt-level = 3`, `lto`, `codegen-units`, `strip`, `panic`
- RUSTC_WRAPPER (sccache) for faster rebuilds

**Output**: CONFIG_REVIEW.md

---

## Phase 5B: MCP_CONFIG.json Validation

**Objective**: Validate MCP configuration

### Check:
- JSON syntax validity
- File paths exist
- Environment variable expansions (e.g., `${GITHUB_PERSONAL_ACCESS_TOKEN}`)
- Settings: `auto_register_tools`, `tool_timeout_secs`, `max_concurrent_calls`

**Output**: Section in CONFIG_REVIEW.md

---

## Phase 5C: Launch Scripts Validation

**Objective**: Review PowerShell launch scripts

### Scripts:
- `launch-gemma2.ps1`
- `launch-qwen-coder.ps1`
- `launch-qwen-fast.ps1`
- `launch-qwen7b.ps1`
- `start-mistralrs.ps1`

### Verify:
- Binary path: `target\release\mistralrs-server.exe`
- Model paths match MODEL_INVENTORY.json
- CUDA environment variables correct

**Output**: Notes in CONFIG_REVIEW.md

---

## Phase 5D: Environment Variables Review

**Objective**: Document required environment setup

### Variables to Check:
- `NVCC_CCBIN`, `CUDA_PATH`, `CUDNN_PATH`
- `MKLROOT`, `HF_HOME`
- Tool availability: node, npx, bun, uv, python, redis-cli

**Output**: Environment section in CONFIG_REVIEW.md

---

## Phase 6: HTTP Server Testing (No MCP)

**Objective**: Test model loading and inference via HTTP API

### Commands:
```powershell
# Start server
& $exe --port 11434 gguf -m (Split-Path $textModel -Parent) -f $textModel 2>&1 | Tee-Object "$logDir\server-no-mcp.log"

# Test inference (in new terminal)
$body = @{
  model = "local"
  messages = @(@{ role="user"; content="Say 'ready' and the current year only." })
  max_tokens = 32
} | ConvertTo-Json -Depth 6

$ttfbSw = [System.Diagnostics.Stopwatch]::StartNew()
$resp = Invoke-RestMethod -Uri "http://localhost:11434/v1/chat/completions" -Method Post -ContentType "application/json" -Body $body
$ttfbSw.Stop()

"TTFB_ms=$($ttfbSw.ElapsedMilliseconds)" | Tee-Object "$logDir\http-no-mcp-ttfb.log"
$resp | ConvertTo-Json -Depth 6 | Out-File -Encoding utf8 "$logDir\http-no-mcp-response.json"

# Monitor VRAM
nvidia-smi dmon -s mu -d 2 -c 5 2>&1 | Tee-Object "$logDir\nvidia-dmon-no-mcp.log"
```

**Metrics**: Load time, VRAM usage, TTFB

---

## Phase 6B: HTTP Server with MCP

**Objective**: Test tool calling via HTTP API

### Commands:
```powershell
& $exe --port 11434 --mcp-config MCP_CONFIG.json gguf -m (Split-Path $textModel -Parent) -f $textModel 2>&1 | Tee-Object "$logDir\server-with-mcp.log"

$body = @{
  model = "local"
  messages = @(
    @{ role="system"; content="You can call tools if needed." },
    @{ role="user"; content="Fetch https://example.com and tell me the HTTP status code." }
  )
  max_tokens = 64
} | ConvertTo-Json -Depth 6

$resp = Invoke-RestMethod -Uri "http://localhost:11434/v1/chat/completions" -Method Post -ContentType "application/json" -Body $body
$resp | ConvertTo-Json -Depth 6 | Out-File -Encoding utf8 "$logDir\http-with-mcp-response.json"
```

**Validation**: MCP tools register and at least one tool call succeeds

---

## Phase 6C: Performance Benchmarking

**Objective**: Compare no-MCP vs MCP-enabled performance

### Metrics:
- Time-to-first-byte (TTFB)
- Tokens per second
- VRAM usage
- Startup time

**Output**: Performance table in COMPREHENSIVE_TEST_REPORT.md

---

## Phase 7: End-to-End Integration Tests

**Objective**: Test real-world scenarios with TUI + MCP

### Scenarios:
1. **Filesystem**: List repo files, read a file
2. **Fetch**: Retrieve example.com, summarize headers
3. **Memory**: Add fact, restart, verify persistence
4. **Sequential Thinking**: Request step-by-step plan
5. **RAG-Redis**: Submit question triggering retrieval

**Output**: Success/failure per tool in COMPREHENSIVE_TEST_REPORT.md

---

## Phase 7B: Multi-Model Switching

**Objective**: Validate model swapping without memory leaks

### Test:
- Load small model
- Monitor VRAM
- Unload and load medium model
- Verify no orphaned GPU memory

**Output**: VRAM deltas in COMPREHENSIVE_TEST_REPORT.md

---

## Phase 7C: Comprehensive Test Suite

**Objective**: Run existing comprehensive test script

### Command:
```powershell
if (Test-Path .\run-tests.ps1) {
  .\run-tests.ps1 2>&1 | Tee-Object "$logDir\run-tests.log"
}
```

**Output**: Pass/fail summary in COMPREHENSIVE_TEST_REPORT.md

---

## Phase 8: Documentation & Consolidation

**Objective**: Generate comprehensive reports and update TODO.md

### Deliverables:

1. **COMPREHENSIVE_TEST_REPORT.md**
   - Executive summary (pass/fail rates)
   - Environment summary
   - Performance metrics
   - Known issues with reproduction steps

2. **MCP_TEST_RESULTS.json**
   - Per-server results with status, timing, errors

3. **TUI_TEST_LOG.txt**
   - Complete TUI session logs

4. **PYO3_STATUS_REPORT.md**
   - Build status, features, API surface

5. **CONFIG_REVIEW.md**
   - Cargo analysis, MCP config, scripts, env vars

6. **TODO.md** (Updated)
   - New issues discovered
   - File paths and suggested fixes

### Final Cleanup:
```powershell
Get-Process mistralrs-server,node,bun,python,rag-redis-mcp-server -ErrorAction SilentlyContinue | Stop-Process -Force
Stop-Transcript
```

---

## Execution Notes

### Directory Structure After Tests:
```
T:\projects\rust-mistral\mistral.rs\
├── .testlogs/                      # All raw logs
│   ├── session-*.log               # PowerShell transcript
│   ├── preflight-*.log
│   ├── mcp-*.log
│   ├── server-*.log
│   └── pyo3-*.log
├── COMPREHENSIVE_TEST_REPORT.md    # Main report
├── MCP_TEST_RESULTS.json          # MCP results
├── TUI_TEST_LOG.txt               # TUI logs
├── PYO3_STATUS_REPORT.md          # PyO3 analysis
├── CONFIG_REVIEW.md               # Config findings
├── MODEL_INVENTORY.json           # Model catalog
└── TODO.md                        # Updated tasks
```

### Execution Time Estimate:
- **Phase 0-1**: 5 minutes (setup, preflight)
- **Phase 2**: 15-20 minutes (MCP server tests)
- **Phase 3**: 10 minutes (TUI tests)
- **Phase 4**: 20-30 minutes (PyO3 build if needed)
- **Phase 5**: 10 minutes (config review)
- **Phase 6**: 15 minutes (HTTP server tests)
- **Phase 7**: 15 minutes (integration tests)
- **Phase 8**: 10 minutes (documentation)

**Total**: ~2-2.5 hours for complete execution

### Prerequisites:
- ✅ mistralrs-server binary built
- ✅ At least one GGUF model available
- ✅ Node.js/npx or Bun installed
- ✅ Python 3.10+ with uv
- ⚠️ Redis optional (for RAG-Redis tests)
- ⚠️ GITHUB_PERSONAL_ACCESS_TOKEN optional (for GitHub MCP)

---

## Quick Start Commands

To begin testing immediately:

```powershell
# Navigate to project
Set-Location 'T:\projects\rust-mistral\mistral.rs'

# Setup (Phase 0 - already complete)
$logDir = Join-Path (Get-Location) '.testlogs'

# Start Phase 1: Pre-flight
.\test-mistralrs.ps1 -Verbose
nvidia-smi
.\setup-dev-env.ps1

# Generate inventory (Phase 1.1)
# Run the MODEL_INVENTORY.json generation script from Phase 1.1

# Test MCP servers (Phase 2)
.\test-mcp-servers.ps1 -Verbose
.\test-phase2-mcp-servers.ps1

# Continue with remaining phases...
```

---

## Support & Troubleshooting

### Common Issues:

1. **Redis not running**: Start with `redis-server` or skip RAG-Redis tests
2. **GitHub MCP fails**: Set `$env:GITHUB_PERSONAL_ACCESS_TOKEN`
3. **PyO3 build errors**: Ensure CUDA headers accessible and Python 3.10+
4. **Model not found**: Run Phase 1.1 to regenerate MODEL_INVENTORY.json
5. **Port in use**: Change test port or kill existing mistralrs-server processes

### Log Analysis:
All logs are timestamped and in `.testlogs/` for easy debugging. Use:
```powershell
Get-ChildItem .testlogs -Filter *.log | Sort-Object LastWriteTime -Descending
```

---

**End of Execution Plan**

This plan provides comprehensive coverage of mistral.rs testing. Execute phases sequentially, capture all outputs, and consolidate findings in the final reports.
