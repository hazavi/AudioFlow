param([switch]$Build)
$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $taskRoot
$taskCargo = Join-Path $taskRoot '.tools\cargo'
if (Test-Path -LiteralPath (Join-Path $taskCargo 'bin\cargo.exe')) {
    $env:CARGO_HOME = $taskCargo
    $env:RUSTUP_HOME = Join-Path $taskRoot '.tools\rustup'
    $env:Path = "$taskCargo\bin;$env:Path"
}
if ($Build) { npm run tauri -- build --bundles nsis } else { npm run desktop }
exit $LASTEXITCODE
