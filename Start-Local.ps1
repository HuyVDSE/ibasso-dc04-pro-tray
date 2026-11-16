$ErrorActionPreference = 'Stop'
$localUrl = 'http://127.0.0.1:8044/'
try {
    $localHealth = Invoke-RestMethod ($localUrl + 'local-health') -TimeoutSec 2
    if ($localHealth.app -ne 'ibasso-local-controller') { throw 'Port 8044 belongs to another application.' }
} catch {
    if (Get-NetTCPConnection -LocalPort 8044 -State Listen -ErrorAction SilentlyContinue) {
        throw 'Port 8044 is already in use. No process was stopped.'
    }
    $pythonExe = (Get-Command python.exe).Source
    $serverScript = Join-Path $PSScriptRoot 'serve_local.py'
    $serverProcess = Start-Process -FilePath $pythonExe -ArgumentList @('"' + $serverScript + '"') -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $PSScriptRoot 'local-server.log') -RedirectStandardError (Join-Path $PSScriptRoot 'local-server-error.log') -PassThru
    $serverProcess.Id | Set-Content (Join-Path $PSScriptRoot 'local-server.pid')
    $ready = $false
    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        Start-Sleep -Milliseconds 250
        try {
            $localHealth = Invoke-RestMethod ($localUrl + 'local-health') -TimeoutSec 1
            if ($localHealth.app -eq 'ibasso-local-controller') { $ready = $true; break }
        } catch { }
    }
    if (-not $ready) { throw 'Local server did not start. Check local-server-error.log.' }
}
Start-Process $localUrl
Write-Host "Opened $localUrl"
