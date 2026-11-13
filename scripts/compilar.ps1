# Compila PDF Light y genera el instalador en dist\.
# Requisitos: Rust (toolchain MSVC) e Inno Setup 7.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)

# Las bibliotecas incrustan la ruta de sus fuentes en los mensajes de error:
# se sustituye la carpeta del usuario para que no acabe dentro del ejecutable.
$env:RUSTFLAGS = "--remap-path-prefix=$env:USERPROFILE=~"

cargo test --release
if ($LASTEXITCODE -ne 0) { throw 'Los tests han fallado.' }
cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'La compilación ha fallado.' }

& "$PSScriptRoot\descargar-ghostscript.ps1"

$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 7\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 7\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'No se encuentra ISCC.exe (Inno Setup 7).' }
& $iscc /Q installer\pdflight.iss
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup ha fallado.' }
Get-ChildItem dist\*.exe | Select-Object Name, Length
