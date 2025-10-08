$modelsRoot = 'C:\codedev\llm\.models'
$entries = @()

if (Test-Path $modelsRoot) {
    $files = Get-ChildItem -Path $modelsRoot -Recurse -File -Include *.gguf,*.safetensors -ErrorAction SilentlyContinue

    foreach ($f in $files) {
        $entries += [pscustomobject]@{
            name = $f.BaseName
            path = $f.FullName
            format = $f.Extension.TrimStart('.')
            size_bytes = $f.Length
            size_gb = [math]::Round($f.Length / 1GB, 2)
            modified_utc = $f.LastWriteTimeUtc.ToString("o")
        }
    }
}

$entries | ConvertTo-Json -Depth 4 | Out-File -Encoding utf8 T:\projects\rust-mistral\mistral.rs\MODEL_INVENTORY.json
Write-Output "Found $($entries.Count) models"

# Display smallest
$smallest = $entries | Where-Object { $_.name -match 'qwen|gemma|phi' } | Sort-Object size_bytes | Select-Object -First 1
if ($smallest) {
    Write-Output "Smallest text model: $($smallest.name) ($($smallest.size_gb) GB)"
}
