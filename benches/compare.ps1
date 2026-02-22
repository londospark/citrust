# benches/compare.ps1
# Benchmark comparison script: Rust vs Python decryption
#
# Usage: .\benches\compare.ps1 "Test Files\Pokemon Y (USA) (Decrypted).3ds"

param(
    [Parameter(Mandatory=$true)]
    [string]$RomPath
)

# Verify ROM exists
if (-not (Test-Path $RomPath)) {
    Write-Error "ROM file not found: $RomPath"
    exit 1
}

# Verify Python script exists
if (-not (Test-Path "test-fixtures\decrypt3.py")) {
    Write-Error "Python script not found: test-fixtures\decrypt3.py"
    exit 1
}

# Verify Rust binary exists
if (-not (Test-Path ".\target\release\citrust.exe")) {
    Write-Error "Rust binary not found. Run 'cargo build --release' first."
    exit 1
}

Write-Host "=== Citrust Benchmark: Rust vs Python ===" -ForegroundColor Cyan
Write-Host ""

# Get ROM filename
$RomName = Split-Path $RomPath -Leaf
$BaseName = [System.IO.Path]::GetFileNameWithoutExtension($RomName)

# Create copies
$RustCopy = ".\benches\rust-$BaseName.3ds"
$PythonCopy = ".\benches\python-$BaseName.3ds"

Write-Host "Creating ROM copies..." -ForegroundColor Yellow
Copy-Item $RomPath $RustCopy -Force
Copy-Item $RomPath $PythonCopy -Force

Write-Host "ROM: $RomName" -ForegroundColor Green
Write-Host "Size: $((Get-Item $RomPath).Length / 1GB) GB" -ForegroundColor Green
Write-Host ""

# Benchmark Rust
Write-Host "Running Rust decryption..." -ForegroundColor Yellow
$RustStart = Get-Date
& .\target\release\citrust.exe $RustCopy | Out-Null
$RustEnd = Get-Date
$RustTime = ($RustEnd - $RustStart).TotalSeconds

# Benchmark Python
Write-Host "Running Python decryption..." -ForegroundColor Yellow
$PythonStart = Get-Date
& python test-fixtures\decrypt3.py $PythonCopy | Out-Null
$PythonEnd = Get-Date
$PythonTime = ($PythonEnd - $PythonStart).TotalSeconds

# Calculate hashes
Write-Host "Verifying outputs..." -ForegroundColor Yellow
$RustHash = (Get-FileHash $RustCopy -Algorithm SHA256).Hash
$PythonHash = (Get-FileHash $PythonCopy -Algorithm SHA256).Hash

# Display results
Write-Host ""
Write-Host "=== Results ===" -ForegroundColor Cyan
Write-Host "Rust:   $([math]::Round($RustTime, 2))s" -ForegroundColor Green
Write-Host "Python: $([math]::Round($PythonTime, 2))s" -ForegroundColor Green
Write-Host "Speedup: $([math]::Round($PythonTime / $RustTime, 2))x" -ForegroundColor Magenta

Write-Host ""
if ($RustHash -eq $PythonHash) {
    Write-Host "SHA256: MATCH ✓" -ForegroundColor Green
} else {
    Write-Host "SHA256: MISMATCH ✗" -ForegroundColor Red
    Write-Host "  Rust:   $RustHash" -ForegroundColor Red
    Write-Host "  Python: $PythonHash" -ForegroundColor Red
}

# Cleanup
Write-Host ""
Write-Host "Cleaning up..." -ForegroundColor Yellow
Remove-Item $RustCopy -Force
Remove-Item $PythonCopy -Force

Write-Host "Done!" -ForegroundColor Green
