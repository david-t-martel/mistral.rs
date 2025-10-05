# Testing Quick Reference Card

> **Quick commands and tips for mistral.rs testing**

---

## Before You Commit

```bash
# Format code
cargo fmt --all

# Check lints
cargo clippy --workspace --all-targets --fix

# Run tests
cargo test --workspace --all-features
```

**Or use the Makefile:**
```bash
make ci-full  # Run full CI pipeline locally
```

---

## Running Tests

### All Tests
```bash
cargo test --workspace --all-features
```

### Specific Crate
```bash
cargo test -p mistralrs-core
cargo test -p mistralrs-agent-tools
```

### Specific Test
```bash
cargo test test_name
cargo test test_name -- --nocapture  # Show output
```

### Integration Tests Only
```bash
cargo test --test '*'
```

### Doc Tests
```bash
cargo test --doc
```

### With Features
```bash
cargo test --all-features
cargo test --no-default-features
cargo test --features "cuda,metal"
```

---

## Debugging Failed Tests

### Show Output
```bash
cargo test -- --nocapture
```

### With Backtrace
```bash
RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=full cargo test
```

### Single-Threaded (Debug Race Conditions)
```bash
cargo test -- --test-threads=1
```

### Specific Test with Debug
```bash
cargo test test_name -- --nocapture --test-threads=1
```

---

## Writing Tests

### Basic Unit Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_valid_input_returns_ok() {
        // Arrange
        let input = "valid";
        
        // Act
        let result = function(input);
        
        // Assert
        assert!(result.is_ok());
    }
}
```

### Async Test
```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### Test with Error
```rust
#[test]
#[should_panic(expected = "error message")]
fn test_panics_on_invalid() {
    function("invalid");
}
```

### Integration Test
```rust
// tests/integration_test.rs
use my_crate::Engine;

#[test]
fn test_engine_integration() {
    let engine = Engine::new();
    let result = engine.process("input");
    assert!(result.is_ok());
}
```

---

## Test Naming Convention

**Pattern**: `test_<function>_<scenario>_<expected>`

**Examples**:
- `test_parse_valid_json_returns_ok`
- `test_parse_empty_input_returns_error`
- `test_calculate_with_negative_returns_error`
- `test_auth_invalid_credentials_denies_access`

---

## CI Checks

### What CI Runs

1. **quick-check**: Formatting & Clippy (2-3 min)
2. **check**: Build verification (5-7 min)
3. **test**: Full test suite (8-12 min)
4. **coverage**: Code coverage (10-15 min)
5. **docs**: Documentation build (3-5 min)
6. **typos**: Spell checking (1 min)
7. **msrv**: Rust 1.86 compatibility (5-7 min)
8. **security-audit**: Dependency scanning (1 min)
9. **integration**: Integration tests (5-10 min)

### Fix Common CI Failures

**Formatting Issues:**
```bash
cargo fmt --all
```

**Clippy Warnings:**
```bash
cargo clippy --workspace --all-targets --fix
```

**Test Failures:**
```bash
cargo test <failing-test> -- --nocapture
RUST_BACKTRACE=1 cargo test <failing-test>
```

**Doc Build Issues:**
```bash
cargo doc --workspace --all-features --no-deps
```

---

## Useful Commands

### Check Without Building
```bash
cargo check --workspace --all-features
```

### Build Documentation
```bash
cargo doc --workspace --no-deps --open
```

### Run Benchmarks
```bash
cargo bench
```

### Update Dependencies
```bash
cargo update
```

### Check Security
```bash
cargo audit
```

### Clean Build Artifacts
```bash
cargo clean
```

---

## Test Utilities

### Temporary Directories
```rust
use tempfile::TempDir;

#[test]
fn test_with_tempdir() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("file.txt");
    // ... use path ...
    // temp cleaned up automatically
}
```

### Mock Data
```rust
#[cfg(test)]
mod test_utils {
    pub fn create_test_config() -> Config {
        Config {
            field: "test_value",
            ..Default::default()
        }
    }
}
```

### Assert Approximately Equal
```rust
pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
    assert!(
        (a - b).abs() < epsilon,
        "Values not approximately equal: {} vs {}",
        a, b
    );
}
```

---

## Environment Variables

### For Tests
```bash
TESTS_HF_TOKEN=<token> cargo test
```

### For Debugging
```bash
RUST_LOG=debug cargo test
RUST_BACKTRACE=1 cargo test
RUST_TEST_THREADS=1 cargo test
```

---

## Makefile Targets

```bash
make test              # All tests
make test-core         # Test mistralrs-core
make test-agent-tools  # Test mistralrs-agent-tools
make test-winutils     # Test winutils
make ci-full           # Full CI pipeline locally
make fmt               # Format code
make lint              # Run clippy
```

---

## Documentation Links

- **Full Testing Guidelines**: `docs/TESTING_GUIDELINES.md`
- **CI/CD Documentation**: `docs/CI_CD.md`
- **Testing Analysis**: `docs/TESTING_ANALYSIS.md`
- **Rust Testing Book**: https://doc.rust-lang.org/book/ch11-00-testing.html

---

## Quick Tips

✅ **DO**:
- Run tests before committing
- Use descriptive test names
- Test edge cases and errors
- Keep tests fast and isolated
- Use fixtures for complex data

❌ **DON'T**:
- Ignore CI failures
- Push unformatted code
- Skip tests with `#[ignore]` permanently
- Test implementation details
- Use shared mutable state

---

## Getting Help

1. Check test output for error details
2. Read CI logs in GitHub Actions
3. Review documentation (links above)
4. Ask in project discussions
5. Create an issue if stuck

---

**Quick Reference Version**: 1.0  
**Last Updated**: 2025-01-05