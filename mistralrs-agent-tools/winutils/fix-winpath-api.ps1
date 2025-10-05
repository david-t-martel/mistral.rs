# Fix all wrapper code to use correct winpath API

$files = @(
    "derive-utils/bash-wrapper/src/lib.rs",
    "derive-utils/cmd-wrapper/src/lib.rs",
    "derive-utils/pwsh-wrapper/src/lib.rs",
    "derive-utils/fd-wrapper/src/lib.rs",
    "derive-utils/fd-wrapper/src/main.rs",
    "derive-utils/rg-wrapper/src/lib.rs",
    "derive-utils/rg-wrapper/src/main.rs"
)

foreach ($file in $files) {
    $path = "T:\projects\coreutils\winutils\$file"
    if (Test-Path $path) {
        Write-Host "Fixing $file..."
        $content = Get-Content $path -Raw

        # Remove PathContext usage
        $content = $content -replace 'use winpath::\{PathNormalizer, PathContext\};', 'use winpath::PathNormalizer;'
        $content = $content -replace 'use winpath::PathContext;', ''

        # Remove target_context/output_context fields
        $content = $content -replace ',?\s*pub\s+(target|output)_context:\s*PathContext,', ''

        # Remove target_context/output_context from Default impl
        $content = $content -replace ',?\s*(target|output)_context:\s*PathContext::\w+,', ''

        # Replace PathNormalizer::with_context() with PathNormalizer::new()
        $content = $content -replace 'PathNormalizer::with_context\(PathContext::\w+\)', 'PathNormalizer::new()'

        # Remove From<ContextArg> for PathContext impls
        $content = $content -replace 'impl From<ContextArg> for PathContext \{[^}]+\}', ''

        # Remove context() methods
        $content = $content -replace 'pub fn (output|target)_context\(mut self, context: PathContext\) -> Self \{[^}]+\}', ''

        Set-Content $path $content -NoNewline
        Write-Host "  Fixed!"
    }
}

Write-Host "`nAll files fixed!"
