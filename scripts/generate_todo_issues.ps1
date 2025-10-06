#!/usr/bin/env pwsh
# Generate GitHub issues from TODO_ANALYSIS.md
# Usage: .\generate_todo_issues.ps1 [-DryRun] [-Priority Critical|High|Medium|Low]

param(
    [switch]$DryRun = $false,
    [ValidateSet("Critical", "High", "Medium", "Low", "All")]
    [string]$Priority = "Critical"
)

$ErrorActionPreference = "Stop"

# TODO tracking data from TODO_ANALYSIS.md
$criticalTodos = @(
    @{
        Title = "[TODO] Implement BitsAndBytes quantization dequantize_w"
        File = "mistralrs-quant/src/bitsandbytes/mod.rs"
        Line = "125"
        Module = "BlockwiseFP8Linear"
        Description = "Core BnB quantization method not implemented - will panic at runtime"
        Solution = "Implement dequantize_w method or document as experimental and remove"
        Priority = "Critical"
        Impact = "BnB quantization completely unusable"
        Phase = "1"
        Item = "1"
    },
    @{
        Title = "[TODO] Implement AFQ quantization methods"
        File = "mistralrs-quant/src/afq/mod.rs"
        Line = "N/A"
        Module = "AfqLayer"
        Description = "AFQ quantization unimplemented"
        Solution = "Complete AFQ implementation or document as experimental"
        Priority = "Critical"
        Impact = "AFQ format completely unusable"
        Phase = "1"
        Item = "2"
    },
    @{
        Title = "[TODO] Add weight accessor for QLoraLinear"
        File = "mistralrs-core/src/lora/qloralinear.rs"
        Line = "N/A"
        Module = "QLoraLinear"
        Description = "Cannot access underlying quantized weights - unimplemented!()"
        Solution = "Add weight() or get_weight() method returning Option<&Tensor>"
        Priority = "Critical"
        Impact = "QLoRA introspection and debugging impossible"
        Phase = "1"
        Item = "3"
    },
    @{
        Title = "[TODO] Add XLora forward pass for DeepSeek2"
        File = "mistralrs-core/src/models/deepseek2.rs"
        Line = "N/A"
        Module = "DeepSeek2Model"
        Description = "XLora not supported - unimplemented!()"
        Solution = "Implement xlora_forward or return proper error"
        Priority = "Critical"
        Impact = "XLora + DeepSeek2 combination will panic"
        Phase = "1"
        Item = "4"
    },
    @{
        Title = "[TODO] Add XLora forward pass for DeepSeek3"
        File = "mistralrs-core/src/models/deepseek3.rs"
        Line = "N/A"
        Module = "DeepSeek3Model"
        Description = "XLora not implemented for DeepSeek3"
        Solution = "Implement xlora_forward or return proper error"
        Priority = "Critical"
        Impact = "XLora + DeepSeek3 will fail"
        Phase = "1"
        Item = "5"
    },
    @{
        Title = "[TODO] Handle Linear layer quant_inner gracefully"
        File = "mistralrs-core/src/lora/mod.rs"
        Line = "N/A"
        Module = "LoRA adapter"
        Description = "No quant method for plain Linear - unimplemented! panic"
        Solution = "Return Option<Arc<dyn QuantMethod>> or Result with clear error"
        Priority = "Critical"
        Impact = "Adapter logic may panic unexpectedly"
        Phase = "1"
        Item = "6"
    },
    @{
        Title = "[TODO] Add graceful flash attention feature detection"
        File = "mistralrs-core/src/attention/backends/flash.rs"
        Line = "151"
        Module = "flash_attn"
        Description = "Runtime panic if flash attention called without feature flag"
        Solution = "Return Result with FeatureNotEnabled error, add cfg checks"
        Priority = "Critical"
        Impact = "Runtime crashes for users without flash-attn compiled"
        Phase = "1"
        Item = "7"
    }
)

$highPriorityTodos = @(
    @{
        Title = "[TODO] Implement flash attention for T5 models"
        File = "mistralrs-core/src/diffusion_models/t5/mod.rs"
        Line = "N/A"
        Module = "T5Attention"
        Description = "Not using flash_attn for T5 - performance bottleneck"
        Solution = "Add flash_attn path for T5 self-attention"
        Priority = "High"
        Impact = "2-3x slower inference for T5-based diffusion models"
        Phase = "2"
        Item = "8"
    },
    @{
        Title = "[TODO] Add blockwise FP8 GEMM kernel"
        File = "mistralrs-quant/src/blockwise_fp8/ops.rs"
        Line = "N/A"
        Module = "fp8_blockwise operations"
        Description = "Missing optimized FP8 GEMM - using fallback"
        Solution = "Implement or integrate vendor-optimized FP8 kernel"
        Priority = "High"
        Impact = "10-20% slower FP8 quantization performance"
        Phase = "2"
        Item = "9"
    },
    @{
        Title = "[TODO] Fix multi-token sequence breaker handling"
        File = "mistralrs-core/src/sampler.rs"
        Line = "N/A"
        Module = "Sampler"
        Description = "Hack for multi-token sequences - see koboldcpp PR#982"
        Solution = "Implement proper solution from linked PR"
        Priority = "High"
        Impact = "Incorrect sampling for complex multi-token patterns"
        Phase = "2"
        Item = "10"
    }
)

function New-IssueMarkdown {
    param($Todo)
    
    @"
## 📍 Location
**File**: ``$($Todo.File)``  
**Line**: $($Todo.Line)  
**Function/Module**: ``$($Todo.Module)``

## 🔍 Current State
**Issue Description**:
$($Todo.Description)

## 💡 Proposed Solution
$($Todo.Solution)

## 🔴 Priority
- [x] **$($Todo.Priority)** - $(if ($Todo.Priority -eq "Critical") { "Runtime panic or security issue" } else { "Performance bottleneck or correctness bug" })

## 🔗 Related
- **TODO Report**: References item #$($Todo.Item) in TODO_ANALYSIS.md Phase $($Todo.Phase)
- **Impact**: $($Todo.Impact)

## ✅ Acceptance Criteria
- [ ] Implementation complete
- [ ] Tests added for fixed code
- [ ] TODO comment removed from source
- [ ] Documentation updated (if applicable)
- [ ] TODO_ANALYSIS.md updated

---
**Created from**: TODO_ANALYSIS.md Phase $($Todo.Phase), Item #$($Todo.Item)
"@
}

function Show-Issue {
    param($Todo)
    
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "TITLE: $($Todo.Title)" -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host (New-IssueMarkdown -Todo $Todo)
}

# Select TODOs based on priority
$todosToProcess = @()
switch ($Priority) {
    "Critical" { $todosToProcess = $criticalTodos }
    "High" { $todosToProcess = $highPriorityTodos }
    "All" { $todosToProcess = $criticalTodos + $highPriorityTodos }
    default { $todosToProcess = $criticalTodos }
}

Write-Host "`n🎯 TODO Issue Generation Script" -ForegroundColor Green
Write-Host "================================" -ForegroundColor Green
Write-Host "Priority Filter: $Priority" -ForegroundColor Cyan
Write-Host "TODOs to process: $($todosToProcess.Count)" -ForegroundColor Cyan
Write-Host "Dry Run: $DryRun`n" -ForegroundColor Cyan

if ($DryRun) {
    Write-Host "📄 DRY RUN MODE - Showing issue previews`n" -ForegroundColor Yellow
    
    foreach ($todo in $todosToProcess) {
        Show-Issue -Todo $todo
    }
    
    Write-Host "`n✅ Dry run complete. No issues created." -ForegroundColor Green
    Write-Host "Run without -DryRun to create issues via GitHub CLI (gh)`n" -ForegroundColor Yellow
} else {
    # Check for GitHub CLI
    $ghAvailable = Get-Command gh -ErrorAction SilentlyContinue
    
    if (-not $ghAvailable) {
        Write-Host "❌ GitHub CLI (gh) not found!" -ForegroundColor Red
        Write-Host "Install: https://cli.github.com/`n" -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "🚀 Creating GitHub issues...`n" -ForegroundColor Green
    
    $created = 0
    $failed = 0
    
    foreach ($todo in $todosToProcess) {
        try {
            $body = New-IssueMarkdown -Todo $todo
            
            # Create issue via GitHub CLI
            $result = gh issue create `
                --title $todo.Title `
                --body $body `
                --label "technical-debt,priority-$($todo.Priority.ToLower())" `
                2>&1
            
            if ($LASTEXITCODE -eq 0) {
                $created++
                Write-Host "✅ Created: $($todo.Title)" -ForegroundColor Green
            } else {
                throw "gh command failed: $result"
            }
        } catch {
            $failed++
            Write-Host "❌ Failed: $($todo.Title)" -ForegroundColor Red
            Write-Host "   Error: $_" -ForegroundColor Red
        }
    }
    
    Write-Host "`n========================================" -ForegroundColor Cyan
    Write-Host "📊 Summary:" -ForegroundColor Yellow
    Write-Host "   Created: $created" -ForegroundColor Green
    Write-Host "   Failed:  $failed" -ForegroundColor Red
    Write-Host "========================================`n" -ForegroundColor Cyan
}

Write-Host "💡 Next steps:" -ForegroundColor Yellow
Write-Host "   1. Review created issues on GitHub"
Write-Host "   2. Assign to team members"
Write-Host "   3. Update TODO_ANALYSIS.md with issue links"
Write-Host "   4. Start fixing critical panics!`n"
