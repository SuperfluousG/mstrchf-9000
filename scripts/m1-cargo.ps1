param([Parameter(Mandatory=$true)][string]$Log, [Parameter(ValueFromRemainingArguments=$true)][string[]]$CargoArgs)
$ErrorActionPreference = 'Stop'
$toolBin = 'C:\Users\Gregg\.rustup\toolchains\1.94.0-x86_64-pc-windows-msvc\bin'
$env:PATH = "$toolBin;$env:PATH"
$env:CARGO = Join-Path $toolBin 'cargo.exe'
$env:RUSTC = Join-Path $toolBin 'rustc.exe'
$env:RUSTDOC = Join-Path $toolBin 'rustdoc.exe'
$env:RUSTUP_TOOLCHAIN = '1.94.0-x86_64-pc-windows-msvc'
$env:RUSTUP_AUTO_INSTALL = '0'
$env:RUSTC_WRAPPER = ''
$env:RUSTC_WORKSPACE_WRAPPER = ''
& $env:CARGO @CargoArgs *> $Log
$checkExit = $LASTEXITCODE
Get-Content -LiteralPath $Log | Select-Object -Last 22
exit $checkExit
