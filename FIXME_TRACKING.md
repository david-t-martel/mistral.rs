# FIXME Tracking and Resolution Plan

**Generated**: 2025-10-08  
**Total FIXME markers**: 6 (3 code + 3 documentation examples)

---

## Active FIXMEs in Code

### 1. Sampler Tokenization Hack
**Location**: `mistralrs-core/src/sampler.rs:128`  
**Issue**: Token encoding uses prefix hack for sequence breakers

```rust
// FIXME: This is a hack. See https://github.com/LostRuins/koboldcpp/pull/982
//        for the correct solution which covers multi-token sequence breakers
//        and ambiguous encodings.
.encode_fast(["a", &breaker].concat(), true)
```

**Impact**: Low - Works but not optimal for edge cases  
**Priority**: Medium  
**Proper Solution**: Implement multi-token sequence breaker logic from koboldcpp PR  
**Effort**: 4-6 hours  
**Tracking**: Need to create GitHub issue referencing upstream PR

**Recommendation**: 
- Document current behavior in code comments
- Create issue linking to proper solution
- Implement when multi-token breakers are needed

---

### 2. Diffusion Pipeline Cache Layers
**Location**: `mistralrs-core/src/pipeline/diffusion.rs:221`  
**Issue**: Hardcoded `num_hidden_layers: 1` for caching

```rust
num_hidden_layers: 1, // FIXME(EricLBuehler): we know this is only for caching, so its OK.
```

**Impact**: Low - Works correctly for intended use (diffusion models don't use KV cache)  
**Priority**: Low  
**Proper Solution**: Refactor to indicate "no KV cache" more explicitly  
**Effort**: 2-3 hours  

**Recommendation**: 
- Convert FIXME to explanatory comment
- This is actually correct behavior, not a bug
- Consider renaming to `cache_layers` or similar

**Proposed Change**:
```rust
// Diffusion models don't use KV cache; this value is only for the ModelConfigMetadata
// interface and doesn't affect model behavior. Set to 1 as a sentinel value.
num_hidden_layers: 1,
```

---

### 3. Speech Pipeline Cache Layers
**Location**: `mistralrs-core/src/pipeline/speech.rs:293`  
**Issue**: Same as diffusion - hardcoded layers for caching

```rust
num_hidden_layers: 1, // FIXME(EricLBuehler): we know this is only for caching, so its OK.
```

**Impact**: Low - Works correctly (speech models also don't need KV cache)  
**Priority**: Low  
**Proper Solution**: Same as diffusion - better documentation  
**Effort**: 1 hour (same fix as diffusion)

**Recommendation**: Apply same solution as diffusion pipeline

---

## Documentation Examples (Not Active Code)

### 4-6. rg-wrapper Examples
**Location**: `mistralrs-agent-tools/winutils/derive-utils/rg-wrapper/src/main.rs`  
Lines: 30, 572, 578

**Issue**: None - These are documentation examples showing how to search for FIXME  
**Priority**: N/A  
**Action**: None needed (intentional usage in examples)

---

## Resolution Summary

| FIXME | Location | Priority | Effort | Action |
|-------|----------|----------|--------|--------|
| Tokenization | sampler.rs:128 | Medium | 4-6h | Create issue, implement later |
| Diffusion cache | diffusion.rs:221 | Low | 2h | Improve comment |
| Speech cache | speech.rs:293 | Low | 1h | Improve comment |
| Examples | rg-wrapper | N/A | 0h | No action (docs) |

**Total Effort**: ~8 hours  
**Quick Wins**: 2 (improve comments) - 3 hours  
**Deferred**: 1 (tokenization) - needs upstream coordination

---

## Immediate Actions (This PR)

### ✅ Quick Fix 1: Improve Cache Layer Comments

**diffusion.rs**:
```rust
// Changed from FIXME to explanatory comment
// Diffusion models don't use KV cache; this value is only for the ModelConfigMetadata
// interface and doesn't affect model behavior. Set to 1 as a sentinel value.
num_hidden_layers: 1,
```

**speech.rs**:
```rust
// Changed from FIXME to explanatory comment  
// Speech models don't use KV cache; this value is only for the ModelConfigMetadata
// interface and doesn't affect model behavior. Set to 1 as a sentinel value.
num_hidden_layers: 1,
```

**Impact**: Better code clarity, removes spurious FIXME markers

---

## Follow-up Actions (Next PR)

### 🔄 Issue Creation: Sampler Tokenization

Create GitHub issue:

**Title**: Improve sequence breaker tokenization in sampler

**Description**:
```
Currently using prefix hack ('a' + breaker) for encoding sequence breakers.

Reference: https://github.com/LostRuins/koboldcpp/pull/982

Proper solution should handle:
- Multi-token sequence breakers
- Ambiguous encodings
- Edge cases in tokenization

Location: mistralrs-core/src/sampler.rs:128

Priority: Medium
Effort: 4-6 hours
```

---

## Long-term Improvements

### Caching Architecture Refactor

Consider creating explicit types for cache configuration:

```rust
enum CacheStrategy {
    KVCache { num_layers: usize },
    NoCache,
    PrefixCache { num_layers: usize },
}

struct ModelConfigMetadata {
    cache_strategy: CacheStrategy,
    // ... other fields
}
```

**Benefits**:
- Type-safe cache configuration
- No magic numbers
- Clear intent in code

**Effort**: 16-24 hours (touches many files)  
**Priority**: Low (current approach works fine)

---

## Recommendations

1. **Immediate**: Improve comments for cache layer FIXMEs ✅ (Done)
2. **This week**: Create issue for tokenization FIXME
3. **This month**: Implement tokenization improvement
4. **This quarter**: Consider cache architecture refactor if other refactoring needed

---

**Status**: 2/3 active FIXMEs resolved (comments improved)  
**Remaining**: 1 (tokenization - issue to be created)  
**Next Review**: After tokenization issue is created

---

*Document created: 2025-10-08*  
*Part of: Quick wins implementation*
