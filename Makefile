# Generate code coverage reports locally
# Note: Coverage builds use local target/ directory and disable sccache

# Helper to ensure local target directory (Windows)
set-local-target:
	@pwsh -Command "$$env:CARGO_TARGET_DIR=''; Write-Host 'Using local target directory'"

test-coverage:
	@echo "Generating code coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --html"
	@echo "Coverage report generated in target/llvm-cov/html/index.html"

test-coverage-open:
	@echo "Generating and opening code coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --open"

test-coverage-lcov:
	@echo "Generating LCOV coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info"
	@echo "LCOV report generated: lcov.info"

test-coverage-json:
	@echo "Generating JSON coverage report..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --json --output-path coverage.json"
	@echo "JSON report generated: coverage.json"

test-coverage-text:
	@echo "Generating text coverage summary..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --summary-only"

test-coverage-ci:
	@echo "Generating coverage for CI (LCOV format)..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info"

test-coverage-fast:
	@echo "Fast coverage (no pyo3 crates)..."
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov -p mistralrs-core -p mistralrs-agent-tools -p mistralrs-quant -p mistralrs-vision -p mistralrs-mcp --all-features --html --open"

install-coverage-tools:
	@echo "Installing code coverage tools..."
	rustup component add llvm-tools-preview
	cargo install cargo-llvm-cov
	@echo "Coverage tools installed!"
