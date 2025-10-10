# Unwrap() Elimination Campaign - Strategy & Progress

**Started**: 2025-10-08  
**Total unwrap() calls**: 2,674  
**Target**: Eliminate all user-facing and critical path unwraps

---

## Distribution Analysis

| Crate | Count | Priority | Status |
|-------|-------|----------|--------|
| mistralrs-agent-tools | 1,095 | Low | 🔵 Deferred (dev tools) |
| mistralrs-core | 994 | High | 🔴 In Progress |
| mistralrs-quant | 190 | Medium | 🟡 Queued |
| mistralrs | 83 | **Critical** | 🔴 **ACTIVE** |
| mistralrs-mcp | 67 | Medium | 🟡 Queued |
| mistralrs-server | 64 | **Critical** | 🔴 **ACTIVE** |
| mistralrs-tui | 43 | Low | 🔵 Deferred |
| mistralrs-paged-attn | 37 | Medium | 🟡 Queued |
| mistralrs-server-core | 33 | High | 🟡 Queued |
| mistralrs-pyo3 | 25 | High | 🟡 Queued |
| Others | 43 | Low | 🔵 Deferred |

---

## Strategy

### Phase 1: Critical User-Facing APIs (Priority: CRITICAL)
**Target**: 147 unwraps (mistralrs + mistralrs-server)  
**Impact**: Direct user experience  
**Approach**: 
1. Analyze each unwrap context
2. Replace with proper error handling
3. Add error context where appropriate
4. Maintain backward compatibility

### Phase 2: Core Library (Priority: HIGH)  
**Target**: 994 unwraps (mistralrs-core)  
**Impact**: Affects all downstream users  
**Approach**:
1. Focus on hot paths (model loading, inference)
2. Public API methods first
3. Internal helpers last

### Phase 3: Supporting Crates (Priority: MEDIUM)
**Target**: 348 unwraps (quant, mcp, paged-attn, server-core, pyo3)  
**Impact**: Feature-specific  
**Approach**: Fix as features are touched

### Phase 4: Development Tools (Priority: LOW)
**Target**: 1,185 unwraps (agent-tools, tui, bench, etc.)  
**Impact**: Developer-only  
**Approach**: Fix opportunistically

---

## Unwrap() Categories & Solutions

### Category 1: Device Selection
**Pattern**: `best_device().unwrap()`, `device.unwrap_or(...).unwrap()`  
**Issue**: Device initialization can fail  
**Solution**: Proper error propagation

```rust
// Before
&self.device.unwrap_or(best_device(self.force_cpu).unwrap())

// After
&self.device.unwrap_or_else(|| best_device(self.force_cpu)
    .expect("Failed to initialize device - no GPU or CPU available"))
```

**Or better with Result**:
```rust
fn get_device(&self) -> anyhow::Result<Device> {
    match &self.device {
        Some(d) => Ok(d.clone()),
        None => best_device(self.force_cpu)
            .context("Failed to initialize device")
    }
}
```

### Category 2: Builder Patterns
**Pattern**: `.build().unwrap()`  
**Issue**: Builder validation can fail  
**Solution**: Return Result from builder methods

```rust
// Before
loader.build().unwrap()

// After  
loader.build()
    .context("Failed to build model loader")?
```

### Category 3: JSON Serialization
**Pattern**: `serde_json::to_value().unwrap()`  
**Issue**: Serialization can fail for large/complex types  
**Solution**: Handle serialization errors

```rust
// Before
result: Some(serde_json::to_value(&data).unwrap())

// After
result: Some(serde_json::to_value(&data)
    .map_err(|e| format!("Serialization error: {}", e))?)
```

### Category 4: Configuration Access
**Pattern**: `config().unwrap()`  
**Issue**: Configuration may not be initialized  
**Solution**: Proper error handling (already started)

```rust
// Before
let config = model.config().unwrap();

// After
let config = model.config()
    .map_err(|e| anyhow!("Failed to get model config: {}", e))?;
```

### Category 5: Lock Acquisition  
**Pattern**: `mutex.lock().unwrap()`, `rwlock.read().unwrap()`  
**Issue**: Lock poisoning can occur  
**Solution**: Handle poisoned locks

```rust
// Before
let data = mutex.lock().unwrap();

// After
let data = mutex.lock()
    .unwrap_or_else(|poisoned| {
        tracing::warn!("Lock was poisoned, recovering");
        poisoned.into_inner()
    });
```

### Category 6: File I/O (History, etc.)
**Pattern**: `save_history().unwrap()`  
**Issue**: File operations can fail  
**Solution**: Log errors, don't panic

```rust
// Before
editor.save_history(&path).unwrap();

// After
if let Err(e) = editor.save_history(&path) {
    tracing::warn!("Failed to save history: {}", e);
}
```

---

## Progress Tracking

### Session 1: Initial Analysis
- [x] Distribution analysis completed
- [x] Prioritization strategy defined
- [x] Category classification done
- [ ] Critical fixes implementation

### Session 2: Critical APIs (Target: 50-100 fixes)
- [ ] mistralrs device selection
- [ ] mistralrs builder patterns  
- [ ] mistralrs-server JSON serialization
- [ ] mistralrs-server file I/O

### Session 3: Core Library (Target: 100-200 fixes)
- [ ] Model loading paths
- [ ] Inference hot paths
- [ ] Cache management

---

## Fixed Unwraps Log

### 2025-10-08: Quick Wins (Session Start)
- ✅ `mistralrs/src/messages.rs`: 2 unwraps → map_err with context
- **Total fixed**: 2
- **Remaining**: 2,672

### Current Session Progress
*Updates will be added as fixes are applied*

---

## Guidelines for Fixing

1. **Never panic in user-facing code**: Use Result/Option properly
2. **Provide context**: Use `.context()` or `.map_err()` with descriptive messages
3. **Log before failing**: Use `tracing::warn!` for recoverable errors
4. **Consider fallbacks**: Use `.unwrap_or_else()` when appropriate
5. **Test error paths**: Ensure error messages are helpful

---

## Next Steps

1. Fix device selection unwraps (mistralrs/*.rs)
2. Fix server JSON serialization (mistralrs-server/mcp_server.rs)
3. Fix file I/O unwraps (mistralrs-server/agent_mode.rs)
4. Continue through priority list

---

**Goal**: Reduce unwrap() count by 100-200 in this session  
**Focus**: User-facing APIs and critical paths  
**Strategy**: Surgical fixes with comprehensive testing
