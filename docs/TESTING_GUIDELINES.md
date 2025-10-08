# Testing Guidelines for mistral.rs

## Table of Contents

1. [Overview](#overview)
1. [Testing Philosophy](#testing-philosophy)
1. [Test Organization](#test-organization)
1. [Test Types](#test-types)
1. [Writing Tests](#writing-tests)
1. [Test Naming Conventions](#test-naming-conventions)
1. [Test Utilities and Helpers](#test-utilities-and-helpers)
1. [Running Tests](#running-tests)
1. [CI/CD Integration](#cicd-integration)
1. [Troubleshooting](#troubleshooting)
1. [Best Practices](#best-practices)
1. [Examples](#examples)

______________________________________________________________________

## Overview

This document outlines the testing strategy, guidelines, and best practices for the mistral.rs project. All contributors should familiarize themselves with these guidelines before writing or modifying tests.

### Testing Goals

1. **Correctness**: Ensure all code behaves as expected
1. **Regression Prevention**: Catch bugs before they reach production
1. **Documentation**: Tests serve as executable documentation
1. **Refactoring Confidence**: Enable safe code changes
1. **Performance**: Detect performance regressions early

### Key Principles

- **Write tests first** when fixing bugs (TDD for bug fixes)
- **Tests should be fast** and deterministic
- **Tests should be isolated** and independent
- **Tests should be readable** and self-documenting
- **Test behavior, not implementation** details

______________________________________________________________________

## Testing Philosophy

### Test Pyramid

We follow the classic test pyramid:

```
         /\        E2E Tests (Few)
        /  \       - Full system tests
       /    \      - User journey tests  
      /______\     
     /        \    Integration Tests (Some)
    /          \   - Module interactions
   /            \  - API contract tests
  /______________\
 /                \ Unit Tests (Many)
/__________________\ - Function/method tests
                    - Pure logic tests
```

**Ratio**: ~70% unit, ~20% integration, ~10% E2E

### Coverage Goals

- **New Code**: 80% line coverage minimum
- **Critical Paths**: 95%+ line and branch coverage
- **Overall Project**: 70%+ line coverage
- **Public APIs**: 100% coverage of public functions

______________________________________________________________________

## Test Organization

### Directory Structure

```
mistral.rs/
├── mistralrs-core/
│   ├── src/
│   │   ├── lib.rs              # Contains #[cfg(test)] mod tests { ... }
│   │   ├── engine/
│   │   │   ├── mod.rs          # Contains unit tests
│   │   │   └── inference.rs    # Contains unit tests
│   ├── tests/                  # Integration tests
│   │   ├── integration_test.rs
│   │   └── fixtures/
│   │       └── test_data.json
│   └── benches/                # Benchmarks
│       └── inference_bench.rs
│
├── mistralrs-agent-tools/
│   ├── src/
│   │   └── tools/
│   │       └── file/
│   │           └── cat.rs      # Contains unit tests
│   └── tests/
│       └── integration.rs
│
└── tests/                      # Workspace-level integration tests
    ├── agents/
    ├── mcp/
    └── integration/
```

### Test Location Rules

1. **Unit Tests**: Place in the same file as the code, in a `#[cfg(test)]` module
1. **Integration Tests**: Place in the `tests/` directory at crate root
1. **Benchmarks**: Place in the `benches/` directory at crate root
1. **Test Fixtures**: Place in `tests/fixtures/` or near the test file

______________________________________________________________________

## Test Types

### 1. Unit Tests

**Purpose**: Test individual functions, methods, or small units of code in isolation.

**Characteristics**:

- Fast execution (microseconds to milliseconds)
- No external dependencies (filesystem, network, database)
- Use mocks/stubs for dependencies
- Test both happy path and error cases

**Example**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_input() {
        let input = "valid data";
        let result = parse(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_input_returns_error() {
        let input = "";
        let result = parse(input);
        assert!(result.is_err());
    }
}
```

### 2. Integration Tests

**Purpose**: Test interactions between modules, crates, or components.

**Characteristics**:

- Slower than unit tests (milliseconds to seconds)
- May use real dependencies (filesystem, test databases)
- Test API contracts and module boundaries
- Located in `tests/` directory

**Example**:

```rust
// tests/integration_test.rs
use mistralrs_core::Engine;

#[test]
fn test_engine_inference_pipeline() {
    let engine = Engine::new(Config::default());
    let result = engine.infer("test prompt");
    assert!(result.is_ok());
}
```

### 3. End-to-End (E2E) Tests

**Purpose**: Test complete user workflows through the entire system.

**Characteristics**:

- Slowest tests (seconds to minutes)
- Use real external services or mocked versions
- Test from user's perspective
- Located in `tests/e2e/` or similar

**Example**:

```rust
// tests/e2e/agent_workflow.rs
#[tokio::test]
async fn test_agent_completes_task_end_to_end() {
    let agent = Agent::new(...);
    let task = Task::new("Write a file");
    let result = agent.execute(task).await;
    assert!(result.is_success());
}
```

### 4. Property-Based Tests

**Purpose**: Test properties that should hold for any input.

**Characteristics**:

- Uses `proptest` or `quickcheck`
- Generates random inputs
- Tests invariants and properties
- Excellent for finding edge cases

**Example**:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encode_decode_roundtrip(data in ".*") {
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        prop_assert_eq!(data, decoded);
    }
}
```

### 5. Benchmarks

**Purpose**: Measure and track performance over time.

**Characteristics**:

- Uses `criterion` for statistical rigor
- Detects performance regressions
- Located in `benches/` directory

**Example**:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_inference(c: &mut Criterion) {
    c.bench_function("inference", |b| {
        b.iter(|| engine.infer(black_box("test prompt")))
    });
}

criterion_group!(benches, benchmark_inference);
criterion_main!(benches);
```

______________________________________________________________________

## Writing Tests

### Test Structure: AAA Pattern

All tests should follow the **Arrange-Act-Assert (AAA)** pattern:

```rust
#[test]
fn test_example() {
    // Arrange: Set up test data and preconditions
    let input = "test data";
    let expected = ProcessedData { ... };
    let processor = Processor::new();

    // Act: Execute the code under test
    let result = processor.process(input);

    // Assert: Verify the outcome
    assert_eq!(result, expected);
}
```

### Async Tests

For async code, use `#[tokio::test]` or `#[async_std::test]`:

```rust
#[tokio::test]
async fn test_async_operation() {
    // Arrange
    let client = AsyncClient::new();

    // Act
    let result = client.fetch_data().await;

    // Assert
    assert!(result.is_ok());
}
```

### Error Testing

Always test error cases:

```rust
#[test]
#[should_panic(expected = "invalid input")]
fn test_panics_on_invalid_input() {
    process("invalid");
}

#[test]
fn test_returns_error_on_invalid_input() {
    let result = safe_process("invalid");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "invalid input");
}
```

### Testing with Mocks

Use mocks for external dependencies:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use mockall::*;

    mock! {
        Database {
            fn fetch(&self, id: u64) -> Result<Data, Error>;
        }
    }

    #[test]
    fn test_with_mock_database() {
        let mut mock_db = MockDatabase::new();
        mock_db
            .expect_fetch()
            .with(eq(42))
            .returning(|_| Ok(Data::default()));

        let service = Service::new(mock_db);
        let result = service.get_data(42);
        assert!(result.is_ok());
    }
}
```

______________________________________________________________________

## Test Naming Conventions

### General Rules

1. **Be descriptive**: Test names should describe what is being tested and the expected outcome
1. **Use snake_case**: Follow Rust conventions
1. **Start with `test_`**: Makes it easy to identify tests
1. **Include scenario**: Describe the specific case being tested

### Naming Templates

#### Unit Tests

**Template**: `test_<function>_<scenario>_<expected_result>`

```rust
#[test]
fn test_parse_valid_json_returns_ok() { ... }

#[test]
fn test_parse_empty_string_returns_error() { ... }

#[test]
fn test_calculate_sum_with_positive_numbers_returns_correct_total() { ... }
```

#### Integration Tests

**Template**: `test_<feature>_<scenario>` or `<component>_<action>_<outcome>`

```rust
#[test]
fn test_auth_valid_credentials_grants_access() { ... }

#[test]
fn engine_inference_with_large_prompt_succeeds() { ... }
```

#### Property Tests

**Template**: `prop_<property>_holds` or `test_<property>_invariant`

```rust
proptest! {
    #[test]
    fn prop_encode_decode_roundtrip_holds(data in ".*") { ... }
    
    #[test]
    fn test_sort_preserves_element_count(vec in prop::collection::vec(any::<i32>(), 0..100)) { ... }
}
```

______________________________________________________________________

## Test Utilities and Helpers

### Common Test Utilities

Create reusable test utilities in a `test_utils` module:

```rust
// src/test_utils.rs or tests/common/mod.rs
#[cfg(test)]
pub mod test_utils {
    use super::*;

    pub fn create_test_config() -> Config {
        Config {
            model_path: "test/model".into(),
            ..Default::default()
        }
    }

    pub fn create_test_engine() -> Engine {
        Engine::new(create_test_config())
    }

    pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
        assert!((a - b).abs() < epsilon, "Values not approximately equal: {} vs {}", a, b);
    }
}
```

### Test Fixtures

For complex test data, use fixtures:

```rust
// tests/fixtures/mod.rs
pub fn load_test_data(name: &str) -> String {
    let path = format!("tests/fixtures/{}.json", name);
    std::fs::read_to_string(path).expect("Failed to read test fixture")
}

// Usage in tests
#[test]
fn test_with_fixture() {
    let data = load_test_data("sample_prompt");
    let result = process(data);
    assert!(result.is_ok());
}
```

### Temporary Test Data

For tests that need filesystem access:

```rust
use tempfile::TempDir;

#[test]
fn test_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    
    // Perform file operations
    write_file(&file_path, "test data").unwrap();
    let content = read_file(&file_path).unwrap();
    
    assert_eq!(content, "test data");
    // temp_dir automatically cleaned up when dropped
}
```

______________________________________________________________________

## Running Tests

### Local Testing

#### Run All Tests

```bash
cargo test --workspace
```

#### Run Tests for Specific Crate

```bash
cargo test -p mistralrs-core
cargo test -p mistralrs-agent-tools
```

#### Run Specific Test

```bash
cargo test test_parse_valid_input
```

#### Run Tests Matching Pattern

```bash
cargo test parse  # Runs all tests with "parse" in the name
```

#### Run with Output

```bash
cargo test -- --nocapture  # Show println! output
cargo test -- --show-output # Show output even for passing tests
```

#### Run Single-Threaded

```bash
cargo test -- --test-threads=1
```

#### Run Integration Tests Only

```bash
cargo test --test '*'
```

#### Run Doc Tests

```bash
cargo test --doc
```

### Testing with Features

```bash
cargo test --all-features
cargo test --no-default-features
cargo test --features "cuda,metal"
```

### Running Benchmarks

```bash
cargo bench
cargo bench --bench inference_bench
```

### Using Makefile Targets

```bash
make test              # Run all tests
make test-core         # Test core crate
make test-agent-tools  # Test agent-tools crate
make test-winutils     # Test winutils
make ci-full           # Run full CI pipeline locally
```

______________________________________________________________________

## CI/CD Integration

### GitHub Actions Workflow

The project uses GitHub Actions for CI/CD. See `.github/workflows/ci.yml` for the full configuration.

#### CI Stages

1. **Quick Check** (Fail Fast)

   - Formatting check (`cargo fmt`)
   - Clippy linting (`cargo clippy`)

1. **Build & Test** (Parallel)

   - Cross-platform check (Linux, macOS, Windows)
   - Workspace tests with all features
   - Documentation tests

1. **Coverage**

   - Code coverage collection (Linux only)
   - Upload to Codecov

1. **Quality Checks**

   - Documentation build
   - Typo checking
   - MSRV verification
   - Security audit

1. **Integration Tests**

   - Full integration test suite

#### Running CI Locally

You can replicate CI checks locally:

```bash
# Quick checks
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Full test suite
cargo test --workspace --all-features

# Documentation
cargo doc --workspace --all-features --no-deps

# Or use the Makefile
make ci-full
```

### Pre-commit Hooks

The project uses pre-commit hooks to catch issues early. Install them:

```bash
pip install pre-commit
pre-commit install
```

Pre-commit runs automatically on `git commit`. To run manually:

```bash
pre-commit run --all-files
```

______________________________________________________________________

## Troubleshooting

### Common Issues

#### Tests Fail Due to Missing Environment Variables

**Problem**: Some tests require environment variables (e.g., `TESTS_HF_TOKEN`).

**Solution**:

```bash
export TESTS_HF_TOKEN="your_token_here"
cargo test
```

#### Tests Are Slow

**Problem**: Tests take a long time to run.

**Solutions**:

1. Run tests in parallel (default):
   ```bash
   cargo test -- --test-threads=4
   ```
1. Run only specific tests:
   ```bash
   cargo test test_name
   ```
1. Use release mode for faster execution:
   ```bash
   cargo test --release
   ```

#### Test Flakiness

**Problem**: Tests pass sometimes and fail other times.

**Solutions**:

1. Ensure tests are isolated (no shared state)
1. Use explicit timeouts for async operations
1. Avoid relying on timing-dependent behavior
1. Use deterministic random seeds:
   ```rust
   use rand::SeedableRng;
   let mut rng = rand::rngs::StdRng::seed_from_u64(42);
   ```

#### Windows-Specific Test Failures

**Problem**: Tests fail on Windows but pass on Linux/macOS.

**Common causes**:

1. Path separator differences (`/` vs `\`)
1. Line ending differences (`\n` vs `\r\n`)
1. Case-sensitive filesystem assumptions
1. Permission handling differences

**Solutions**:

- Use `std::path::Path` and `PathBuf` for paths
- Normalize line endings in tests
- Use platform-specific conditional compilation:
  ```rust
  #[cfg(target_os = "windows")]
  #[test]
  fn test_windows_specific() { ... }
  ```

### Debugging Failed Tests

#### Print Debug Information

```rust
#[test]
fn test_with_debug() {
    let value = complex_computation();
    dbg!(&value);  // Prints value to stderr
    assert_eq!(value, expected);
}
```

#### Use `cargo test -- --nocapture`

Shows `println!` and `eprintln!` output even for passing tests.

#### Run Tests with Backtraces

```bash
RUST_BACKTRACE=1 cargo test
RUST_BACKTRACE=full cargo test  # More detailed
```

#### Use `cargo test -- --exact`

Run tests matching exact name:

```bash
cargo test test_specific_function --exact
```

______________________________________________________________________

## Best Practices

### DO

✅ **Write tests for bug fixes**

- Add a failing test that reproduces the bug
- Fix the bug
- Verify the test now passes

✅ **Test edge cases**

- Empty inputs
- Boundary values (0, max, min)
- Invalid inputs
- Large inputs

✅ **Keep tests simple and focused**

- One assertion per test (when possible)
- Test one thing at a time
- Avoid complex setup

✅ **Use descriptive assertion messages**

```rust
assert_eq!(result, expected, "Expected greeting to contain name");
```

✅ **Make tests deterministic**

- Avoid random values (or use seeded RNG)
- Avoid timing-dependent behavior
- Isolate tests from each other

✅ **Test public APIs thoroughly**

- Every public function should have tests
- Test all documented behavior
- Test error conditions

### DON'T

❌ **Don't test implementation details**

- Test behavior, not internal structure
- Refactoring shouldn't break tests

❌ **Don't write slow tests**

- Unit tests should be fast (< 10ms)
- Use mocks instead of real I/O
- Run expensive tests separately

❌ **Don't use shared mutable state**

- Tests should be independent
- Avoid static mut, global state
- Use test isolation techniques

❌ **Don't ignore test failures**

- Fix failing tests immediately
- Don't skip tests with `#[ignore]` permanently
- Don't comment out failing tests

❌ **Don't test third-party code**

- Assume libraries work correctly
- Test your usage of libraries
- Use integration tests for external dependencies

______________________________________________________________________

## Examples

### Complete Test Module Example

```rust
// src/parser.rs

pub struct Parser {
    strict: bool,
}

impl Parser {
    pub fn new(strict: bool) -> Self {
        Self { strict }
    }

    pub fn parse(&self, input: &str) -> Result<ParsedData, ParseError> {
        if input.is_empty() {
            return Err(ParseError::EmptyInput);
        }
        // ... parsing logic
        Ok(ParsedData { /* ... */ })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function for tests
    fn create_parser() -> Parser {
        Parser::new(false)
    }

    #[test]
    fn test_new_creates_parser_with_correct_settings() {
        let parser = Parser::new(true);
        assert!(parser.strict);
    }

    #[test]
    fn test_parse_valid_input_returns_ok() {
        let parser = create_parser();
        let result = parser.parse("valid input");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_empty_input_returns_error() {
        let parser = create_parser();
        let result = parser.parse("");
        assert!(matches!(result, Err(ParseError::EmptyInput)));
    }

    #[test]
    #[should_panic(expected = "invalid format")]
    fn test_strict_parser_panics_on_invalid_format() {
        let parser = Parser::new(true);
        parser.parse("bad format").unwrap();
    }
}
```

### Integration Test Example

```rust
// tests/integration_test.rs

use mistralrs_core::{Engine, Config};

#[tokio::test]
async fn test_engine_inference_flow() {
    // Arrange
    let config = Config {
        model_path: "test_model".into(),
        ..Default::default()
    };
    let engine = Engine::new(config).await.expect("Failed to create engine");

    // Act
    let prompt = "Hello, world!";
    let result = engine.infer(prompt).await;

    // Assert
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.is_empty());
}
```

______________________________________________________________________

## Summary

- **Follow the test pyramid**: Many unit tests, fewer integration tests, few E2E tests
- **Use descriptive names**: Test names should explain what is being tested
- **Keep tests fast**: Unit tests should be < 10ms
- **Test edge cases**: Empty inputs, boundaries, invalid data
- **Write tests for bugs**: Reproduce bug, fix, verify
- **Run tests frequently**: Before commit, during development
- **Maintain test quality**: Tests should be as clean as production code

**For more information**:

- See `docs/CI_CD.md` for CI/CD pipeline details
- See `docs/TESTING_ANALYSIS.md` for testing infrastructure analysis
- See Rust Testing Book: https://doc.rust-lang.org/book/ch11-00-testing.html

______________________________________________________________________

*Document Version*: 1.0\
*Last Updated*: 2025-01-05\
*Maintained by*: Testing Infrastructure Team
