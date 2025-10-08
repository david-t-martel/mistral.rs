# Test Fixtures

This directory contains test fixtures for mistralrs-agent-tools tests.

## Organization

```
fixtures/
├── configs/          # Configuration files for testing
├── text/             # Text files for processing tests
├── binary/           # Binary files for testing
├── models/           # Model configuration files
└── prompts/          # Sample prompts for testing
```

## Usage

Load fixtures in tests using the `load_fixture()` utility:

```rust
use mistralrs_agent_tools::test_utils::load_fixture;

#[test]
fn test_with_fixture() {
    let data = load_fixture("configs/sample.json");
    // Use data in test...
}
```

## Adding New Fixtures

1. Place fixture files in the appropriate subdirectory
1. Use descriptive names: `sample_<purpose>.<ext>`
1. Keep fixtures small (< 1MB)
1. Document complex fixtures with comments or README

## Fixture Guidelines

- **Text files**: UTF-8 encoded, LF line endings
- **JSON files**: Formatted with 2-space indentation
- **Binary files**: Keep minimal size
- **Naming**: lowercase with underscores

## Examples

- `text/sample_input.txt` - Simple text file
- `configs/sample_config.json` - Sample configuration
- `prompts/sample_prompt.txt` - Sample prompt text
