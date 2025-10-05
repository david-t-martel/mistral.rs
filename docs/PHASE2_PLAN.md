# Phase 2: Coverage & Measurement - Implementation Plan

## Status: IN PROGRESS
**Start Date**: 2025-01-05  
**Expected Completion**: Week 2  
**Priority**: HIGH

---

## Overview

Phase 2 focuses on establishing code coverage measurement, setting baseline metrics, and standardizing test organization across the project.

---

## Objectives

1. **Set up code coverage infrastructure** with cargo-llvm-cov
2. **Establish baseline coverage metrics** for all crates
3. **Standardize test organization** and naming conventions
4. **Document test fixtures** and test data management
5. **Create test helper utilities** for common patterns

---

## Tasks

### Task 2.1: Generate Baseline Coverage Report ✓

**Status**: COMPLETE  
**Owner**: Testing Team  
**Priority**: HIGH  

**Steps**:
1. ✅ Install coverage tools (`cargo install cargo-llvm-cov`)
2. ✅ Run coverage on current codebase
3. ✅ Generate HTML and LCOV reports
4. ✅ Document current coverage per crate
5. ✅ Identify coverage gaps

**Command**:
```bash
make install-coverage-tools
make test-coverage-text
make test-coverage-open
```

**Deliverables**:
- [ ] Baseline coverage report (HTML)
- [ ] Coverage metrics spreadsheet
- [ ] Gap analysis document

---

### Task 2.2: Document Current Coverage State

**Status**: PENDING  
**Priority**: HIGH  

**Steps**:
1. [ ] Create coverage metrics document
2. [ ] Document coverage per crate:
   - mistralrs-core
   - mistralrs-agent-tools
   - mistralrs-quant
   - mistralrs-vision
   - mistralrs-audio
   - mistralrs-server
   - mistralrs-mcp
   - mistralrs-tui
3. [ ] Identify critical uncovered code paths
4. [ ] Create action plan for improving coverage

**Deliverables**:
- [ ] `docs/COVERAGE_BASELINE.md`
- [ ] Coverage improvement roadmap

---

### Task 2.3: Standardize Test Organization

**Status**: PENDING  
**Priority**: MEDIUM  

**Steps**:
1. [ ] Audit current test organization
2. [ ] Create test organization standards document
3. [ ] Refactor tests to follow standards:
   - Move inline tests to `#[cfg(test)]` modules
   - Organize integration tests in `tests/` directories
   - Separate test utilities into `test_utils` modules
4. [ ] Update testing guidelines with examples

**Standards**:
- **Unit tests**: In `#[cfg(test)]` mod at end of file
- **Integration tests**: In `tests/` directory
- **Test utilities**: In `tests/common/mod.rs` or `src/test_utils.rs`
- **Test naming**: `test_<function>_<scenario>_<expected>`

**Deliverables**:
- [ ] Test organization audit report
- [ ] Refactored tests following standards
- [ ] Updated TESTING_GUIDELINES.md with examples

---

### Task 2.4: Create Test Helper Utilities

**Status**: PENDING  
**Priority**: MEDIUM  

**Steps**:
1. [ ] Identify common test patterns across crates
2. [ ] Create shared test utilities module:
   - Fixture loading
   - Mock data builders
   - Assertion helpers
   - Temporary file/directory utilities
3. [ ] Document test utilities usage
4. [ ] Refactor existing tests to use utilities

**Example Utilities**:
```rust
// tests/common/mod.rs or src/test_utils.rs
pub fn create_test_config() -> Config { ... }
pub fn load_test_fixture(name: &str) -> String { ... }
pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) { ... }
pub fn create_temp_dir() -> TempDir { ... }
```

**Deliverables**:
- [ ] Test utilities modules per crate
- [ ] Test utilities documentation
- [ ] Usage examples in TESTING_GUIDELINES.md

---

### Task 2.5: Set Up Test Fixtures Framework

**Status**: PENDING  
**Priority**: MEDIUM  

**Steps**:
1. [ ] Create `tests/fixtures/` directories
2. [ ] Document fixture organization:
   - JSON test data
   - Binary test files
   - Configuration files
3. [ ] Create fixture loading utilities
4. [ ] Add fixtures for common test scenarios
5. [ ] Document fixture management

**Directory Structure**:
```
crate/
├── tests/
│   ├── fixtures/
│   │   ├── models/
│   │   ├── configs/
│   │   ├── prompts/
│   │   └── responses/
│   └── integration_test.rs
```

**Deliverables**:
- [ ] Fixture directories created
- [ ] Fixture loading utilities
- [ ] Fixture management documentation

---

### Task 2.6: Improve Coverage for Critical Modules

**Status**: PENDING  
**Priority**: HIGH  

**Steps**:
1. [ ] Identify critical modules (<80% coverage)
2. [ ] Write tests for uncovered code paths:
   - Error handling paths
   - Edge cases
   - Boundary conditions
3. [ ] Verify coverage improvements
4. [ ] Document coverage improvement strategy

**Target Modules** (prioritized):
1. **mistralrs-core** (inference engine, model loading)
2. **mistralrs-agent-tools** (file operations, shell execution)
3. **mistralrs-mcp** (MCP protocol handling)
4. **mistralrs-server** (HTTP API)

**Deliverables**:
- [ ] New tests for critical modules
- [ ] Coverage improvement report
- [ ] 80%+ coverage for critical modules

---

### Task 2.7: Add Coverage Badge to README

**Status**: PENDING (requires Codecov setup)  
**Priority**: LOW  

**Steps**:
1. [ ] Set up Codecov account
2. [ ] Link repository to Codecov
3. [ ] Configure `CODECOV_TOKEN` secret
4. [ ] Test coverage upload in CI
5. [ ] Add badge to README.md
6. [ ] Add CI status badge

**README Badge Example**:
```markdown
[![CI](https://github.com/YOUR_ORG/mistral.rs/workflows/Continuous%20integration/badge.svg)](https://github.com/YOUR_ORG/mistral.rs/actions)
[![codecov](https://codecov.io/gh/YOUR_ORG/mistral.rs/branch/master/graph/badge.svg)](https://codecov.io/gh/YOUR_ORG/mistral.rs)
```

**Deliverables**:
- [ ] Codecov integration complete
- [ ] Coverage badge in README
- [ ] CI badge in README

---

### Task 2.8: Document Testing Best Practices

**Status**: PENDING  
**Priority**: MEDIUM  

**Steps**:
1. [ ] Document test writing best practices:
   - AAA pattern (Arrange-Act-Assert)
   - Test isolation
   - Mocking strategies
   - Error handling tests
2. [ ] Create examples for common patterns
3. [ ] Add troubleshooting section
4. [ ] Review with team

**Deliverables**:
- [ ] Enhanced TESTING_GUIDELINES.md
- [ ] Test examples repository
- [ ] Team training materials (optional)

---

## Success Criteria

### Coverage Metrics
- [ ] **Baseline established**: Coverage metrics documented for all crates
- [ ] **Target set**: 70% overall, 80% for new code
- [ ] **Critical modules**: 80%+ coverage for core, agent-tools, MCP

### Test Organization
- [ ] **Standards defined**: Clear test organization guidelines
- [ ] **Tests refactored**: All tests follow naming conventions
- [ ] **Utilities created**: Common test utilities available

### Documentation
- [ ] **Coverage guide**: CODE_COVERAGE.md complete and accurate
- [ ] **Testing guidelines**: Updated with examples and best practices
- [ ] **Fixtures documented**: Fixture management guide available

### CI Integration
- [ ] **Coverage in CI**: cargo-llvm-cov running in CI
- [ ] **Codecov integrated**: Coverage reports uploading successfully
- [ ] **Badges added**: README shows CI and coverage status

---

## Timeline

### Week 2 Schedule

**Day 1-2**:
- ✅ Install coverage tools
- [ ] Generate baseline coverage report
- [ ] Document current coverage state

**Day 3-4**:
- [ ] Audit test organization
- [ ] Create test helper utilities
- [ ] Set up test fixtures framework

**Day 5-7**:
- [ ] Improve coverage for critical modules
- [ ] Standardize test organization
- [ ] Set up Codecov integration
- [ ] Add badges to README

---

## Risks and Mitigations

### Risk: Low Initial Coverage
**Likelihood**: HIGH  
**Impact**: MEDIUM  
**Mitigation**: Focus on critical modules first, set achievable incremental targets

### Risk: Test Refactoring Takes Longer Than Expected
**Likelihood**: MEDIUM  
**Impact**: MEDIUM  
**Mitigation**: Prioritize critical tests, refactor incrementally

### Risk: Codecov Integration Issues
**Likelihood**: LOW  
**Impact**: LOW  
**Mitigation**: Have fallback plan (local coverage reports only)

---

## Next Steps After Phase 2

**Phase 3: Advanced Testing** (Week 3)
- Property-based testing with proptest
- Fuzzing infrastructure setup
- Benchmark regression detection
- Comprehensive integration test suite

---

## Notes

- Coverage tools already installed via Phase 1
- CI workflow already configured for coverage
- codecov.yml configuration file ready
- Makefile targets for coverage already added

---

**Document Version**: 1.0  
**Last Updated**: 2025-01-05  
**Status**: In Progress  
**Owner**: Testing Infrastructure Team