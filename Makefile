# Generate code coverage reports locally

test-coverage:
	@echo "Generating code coverage report..."
	cargo llvm-cov --workspace --all-features --html
	@echo "Coverage report generated in target/llvm-cov/html/index.html"

test-coverage-open:
	@echo "Generating and opening code coverage report..."
	cargo llvm-cov --workspace --all-features --open

test-coverage-lcov:
	@echo "Generating LCOV coverage report..."
	cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info
	@echo "LCOV report generated: lcov.info"

test-coverage-json:
	@echo "Generating JSON coverage report..."
	cargo llvm-cov --workspace --all-features --json --output-path coverage.json
	@echo "JSON report generated: coverage.json"

test-coverage-text:
	@echo "Generating text coverage summary..."
	cargo llvm-cov --workspace --all-features --summary-only

test-coverage-ci:
	@echo "Generating coverage for CI (LCOV format)..."
	cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

install-coverage-tools:
	@echo "Installing code coverage tools..."
	rustup component add llvm-tools-preview
	cargo install cargo-llvm-cov
	@echo "Coverage tools installed!"
