$ErrorActionPreference = 'Stop'
$pidPath = Join-Path $PSScriptRoot 'local-server.pid'
if (-not (Test-Path -LiteralPath $pidPath)) { Write-Host 'No saved local server PID.'; exit }
$serverPid = [int](Get-Content -LiteralPath $pidPath)
$serverProcess = Get-CimInstance Win32_Process -Filter "ProcessId=$serverPid"
$expectedScript = Join-Path $PSScriptRoot 'serve_local.py'
if ($serverProcess -and $serverProcess.Name -match '^python(w)?\.exe$' -and $serverProcess.CommandLine.Contains($expectedScript)) {
    Stop-Process -Id $serverPid
    Remove-Item -LiteralPath $pidPath
    Write-Host 'Stopped iBasso local server.'
} elseif ($serverProcess) {
    throw 'Saved PID belongs to a different process. No process was stopped.'
} else {
    Remove-Item -LiteralPath $pidPath
    Write-Host 'Local server was already stopped.'
}
