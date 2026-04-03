$ErrorActionPreference = "Stop"

$Repo = "https://github.com/JGabrine/pt.git"
$InstallDir = "$env:LOCALAPPDATA\pt"
$BinDir = $InstallDir

Write-Host "Installing Prompt Tuner..."

# Check dependencies
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: cargo not found. Install Rust first: https://rustup.rs" -ForegroundColor Red
    exit 1
}

if (-not (Get-Command claude -ErrorAction SilentlyContinue)) {
    Write-Host "Warning: Claude Code CLI not found. Install from https://docs.anthropic.com/claude-code" -ForegroundColor Yellow
}

# Clone or update
if (Test-Path "$InstallDir\.git") {
    Write-Host "Updating existing installation..."
    git -C $InstallDir pull --ff-only
} else {
    if (Test-Path $InstallDir) {
        # Directory exists but isn't a repo — remove and re-clone
        Remove-Item -Recurse -Force $InstallDir
    }
    Write-Host "Cloning repository..."
    git clone $Repo $InstallDir
}

# Build
Write-Host "Building..."
cargo build --release --manifest-path "$InstallDir\Cargo.toml"
if ($LASTEXITCODE -ne 0) { exit 1 }

# Copy binary next to repo
Copy-Item "$InstallDir\target\release\pt.exe" "$BinDir\pt.exe" -Force

# Check PATH
if ($env:PATH -notlike "*$BinDir*") {
    Write-Host ""
    Write-Host "Note: $BinDir is not in your PATH." -ForegroundColor Yellow
    Write-Host "Add it with: [Environment]::SetEnvironmentVariable('PATH', `"$BinDir;`$env:PATH`", 'User')"
    Write-Host ""
}

# Register hook
& "$BinDir\pt.exe" --setup

Write-Host ""
Write-Host "Done. Restart Claude Code to activate."
