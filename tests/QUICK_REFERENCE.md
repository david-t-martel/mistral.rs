# Test Runner Quick Reference Card

## 🚀 Common Commands

### Via Make (Recommended)

```bash
make test-ps1-quick        # Quick smoke tests (1 min)
make test-ps1              # Full PowerShell suite (15-20 min)
make test-full             # All tests: Rust + PowerShell
make test-ps1-integration  # Integration tests only
make test-ps1-mcp          # MCP tests only
make test-ps1-ci           # CI mode (strict, JSON)
```

### Direct PowerShell

```powershell
.\tests\run-all-tests.ps1                               # All tests
.\tests\run-all-tests.ps1 -Suite quick                  # Quick (1 min)
.\tests\run-all-tests.ps1 -Suite integration            # Integration
.\tests\run-all-tests.ps1 -Suite mcp                    # MCP
.\tests\run-all-tests.ps1 -OutputFormat html            # HTML report
.\tests\run-all-tests.ps1 -Verbose -FailFast            # Debug mode
.\tests\run-all-tests.ps1 -CI -OutputFormat json        # CI mode
```

## 📊 Output Formats

| Format   | Command                  | Output     | Use Case      |
| -------- | ------------------------ | ---------- | ------------- |
| Console  | `-OutputFormat console`  | Terminal   | Development   |
| JSON     | `-OutputFormat json`     | .json file | CI/CD         |
| Markdown | `-OutputFormat markdown` | .md file   | Documentation |
| HTML     | `-OutputFormat html`     | .html file | Reports       |

## ⏱️ Suite Durations

| Suite       | Duration  | Tests    | Use Case           |
| ----------- | --------- | -------- | ------------------ |
| quick       | ~1 min    | 1        | Pre-commit         |
| integration | 5-10 min  | Variable | Feature validation |
| mcp         | 5-10 min  | Variable | MCP integration    |
| build       | 10-15 min | Variable | Build system       |
| all         | 15-20 min | All      | Full validation    |

## 🔧 Common Options

```powershell
-Verbose          # Detailed output
-FailFast         # Stop on first failure
-CI               # CI mode (no prompts, strict)
-Parallel         # Parallel execution (experimental)
-OutputFile <path> # Custom output location
```

## 📁 Test Locations

```
tests/
├── run-all-tests.ps1          # Master runner
├── integration/               # Integration tests
│   └── test-*.ps1
├── mcp/                       # MCP tests
│   ├── MCP_CONFIG.json
│   └── test-*.ps1
└── results/                   # Test results
    ├── *.json
    ├── *.md
    ├── *.html
    └── mcp-*.{out,err}        # MCP server logs
```

## 🛠️ Troubleshooting

| Issue                   | Quick Fix                                      |
| ----------------------- | ---------------------------------------------- |
| Tests not found         | Run `.\tests\validate-test-runner.ps1`         |
| MCP servers won't start | Check `node --version`                         |
| Binary not found        | Run `make build-cuda-full`                     |
| Permission denied       | Run as Administrator or adjust ExecutionPolicy |
| Tests hang              | Use `-FailFast -Verbose` to debug              |

## 📝 Adding New Tests

1. **Create script**: `tests/<category>/test-<name>.ps1`
1. **Exit codes**: 0 = pass, non-zero = fail
1. **Optional JSON**: Output structured results
1. **Test**: Run `.\tests\run-all-tests.ps1 -Suite <category>`

## 🎯 Development Workflow

```bash
# 1. Pre-commit (always)
make test-ps1-quick

# 2. Feature development
make test-ps1-integration

# 3. Pre-push
make test-full

# 4. Pre-release
.\tests\run-all-tests.ps1 -Suite all -OutputFormat html
```

## 🚨 CI/CD Usage

### GitHub Actions

```yaml
- name: Run Tests
  run: make test-ps1-ci

- name: Upload Results
  uses: actions/upload-artifact@v4
  with:
    name: test-results
    path: tests/results/*.json
```

### Exit Codes

- `0` = All tests passed
- `1` = Tests failed
- Other = Fatal error

## 📚 More Information

- **Full Guide**: `tests/README.md`
- **Implementation Details**: `tests/TEST_RUNNER_IMPLEMENTATION.md`
- **Validation**: `.\tests\validate-test-runner.ps1`

## 💡 Tips

1. ✅ Use `make test-ps1-quick` before every commit
1. ✅ Generate HTML reports for visual review
1. ✅ Use `-FailFast -Verbose` when debugging
1. ✅ Check `tests/results/mcp-*.err` for MCP issues
1. ✅ Archive old results regularly (done automatically)

______________________________________________________________________

**Quick Help**: `make help` or `.\tests\run-all-tests.ps1 -?`
