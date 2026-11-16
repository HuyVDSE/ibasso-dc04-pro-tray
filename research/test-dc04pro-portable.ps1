param(
    [Parameter(Mandatory)]
    [ValidateSet('Launch', 'Baseline', 'Show', 'Restore')]
    [string]$Action,
    [ValidateSet('A', 'B')]
    [string]$Copy = 'A'
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$name = 'IbassoDc04ProTray'
$entries = @(
    'Software\Microsoft\Windows\CurrentVersion\Run'
    'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run'
)
$user = [Security.Principal.WindowsIdentity]::GetCurrent()
$backup = Join-Path $PSScriptRoot "dc04pro-startup-before-$($user.User.Value).clixml"
$hive = [Microsoft.Win32.Registry]::CurrentUser

function Read-Entry([string]$Subkey) {
    $key = $hive.OpenSubKey($Subkey)
    try {
        $present = $null -ne $key -and $name -in $key.GetValueNames()
        [pscustomobject]@{
            Subkey = $Subkey
            Present = $present
            Kind = if ($present) { [int]$key.GetValueKind($name) } else { $null }
            Value = if ($present) {
                ,$key.GetValue($name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            } else { $null }
        }
    } finally {
        if ($null -ne $key) { $key.Dispose() }
    }
}

switch ($Action) {
    'Launch' {
        $principal = [Security.Principal.WindowsPrincipal]::new($user)
        if ($principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
            throw 'Use a regular (non-elevated) PowerShell window.'
        }
        $session = (Get-Process -Id $PID).SessionId
        $running = Get-Process dc04-tray -ErrorAction SilentlyContinue |
            Where-Object SessionId -EQ $session
        if ($running) { throw 'Exit the running tray from its menu, then retry.' }

        $dist = Join-Path $root 'dist\dc04-tray.exe'
        $first = Join-Path $root "Portable Test $Copy\dc04-tray.exe"
        $other = if ($Copy -eq 'A') { 'B' } else { 'A' }
        $second = Join-Path $root "Portable Test $other\dc04-tray.exe"
        $hash = (Get-FileHash -LiteralPath $dist -Algorithm SHA256).Hash
        foreach ($exe in $first, $second) {
            if ((Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $hash) {
                throw "File does not match dist: $exe"
            }
        }
        $one = Start-Process -FilePath $first -WorkingDirectory $PSScriptRoot -PassThru
        $two = $null
        try {
            if ($one.WaitForExit(2500)) { throw "First tray exited with code $($one.ExitCode)." }
            $two = Start-Process -FilePath $second -WorkingDirectory $PSScriptRoot -PassThru
            if (-not $two.WaitForExit(5000)) { throw 'Second test tray did not exit within 5 seconds.' }
            if ($two.ExitCode -ne 0 -or $one.HasExited) { throw 'Singleton check failed.' }
        } catch {
            foreach ($started in $one, $two) {
                if ($null -ne $started -and -not $started.HasExited) {
                    Stop-Process -InputObject $started -ErrorAction SilentlyContinue
                }
            }
            throw
        }
        [pscustomobject]@{
            Path = $first
            SHA256 = $hash
            Session = $session
            TrayPID = $one.Id
            SecondExitCode = $two.ExitCode
            Elevated = $false
        }
    }
    'Baseline' {
        if (Test-Path -LiteralPath $backup) {
            throw "Baseline already exists: $backup. Keep it until startup has been restored."
        }
        $before = @($entries | ForEach-Object { Read-Entry $_ })
        $before | Export-Clixml -LiteralPath $backup -NoClobber
        $before
    }
    'Show' { $entries | ForEach-Object { Read-Entry $_ } }
    'Restore' {
        $before = @(Import-Clixml -LiteralPath $backup)
        foreach ($subkey in $entries) {
            $saved = @($before | Where-Object Subkey -CEQ $subkey)
            if ($saved.Count -ne 1) { throw "Invalid baseline for: $subkey" }
            $key = if ($saved[0].Present) {
                $hive.CreateSubKey($subkey)
            } else {
                $hive.OpenSubKey($subkey, $true)
            }
            try {
                if ($saved[0].Present) {
                    $kind = [Microsoft.Win32.RegistryValueKind]$saved[0].Kind
                    $value = switch ($kind) {
                        'Binary' { ,([byte[]]$saved[0].Value) }
                        'MultiString' { ,([string[]]$saved[0].Value) }
                        'DWord' { [int]$saved[0].Value }
                        'QWord' { [long]$saved[0].Value }
                        default { [string]$saved[0].Value }
                    }
                    $key.SetValue($name, $value, $kind)
                } elseif ($null -ne $key) {
                    $key.DeleteValue($name, $false)
                }
            } finally {
                if ($null -ne $key) { $key.Dispose() }
            }
        }
        $entries | ForEach-Object { Read-Entry $_ }
    }
}
