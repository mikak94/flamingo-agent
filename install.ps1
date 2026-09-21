#Requires -RunAsAdministrator
# Builds both binaries, installs them under Program Files, registers and
# starts the service. -Uninstall reverses it. Re-runnable.
param([switch] $Uninstall)

$ErrorActionPreference = 'Stop'

$ServiceName = 'FlamingoAgent'
$InstallDir  = Join-Path $env:ProgramFiles 'FlamingoAgent'
$DataDir     = Join-Path $env:ProgramData  'FlamingoAgent'
$Agent       = Join-Path $InstallDir 'flamingo-agent.exe'
$ChildDir    = Join-Path $PSScriptRoot 'logger-child'
$BuildDir    = Join-Path $ChildDir 'build'

function Assert-LastExitCode($what) {
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit $LASTEXITCODE)" }
}

if ($Uninstall) {
    if (Test-Path $Agent) { try { & $Agent --uninstall } catch { Write-Warning $_ } }
    Remove-Item $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "uninstalled, logs are still in $DataDir"
    return
}

Write-Host "`n-- building the agent"
cargo build --release --manifest-path (Join-Path $PSScriptRoot 'agent-rust\Cargo.toml')
Assert-LastExitCode 'cargo build'

Write-Host "`n-- building the child"
New-Item -ItemType Directory -Force -Path $BuildDir | Out-Null
$exe = Join-Path $BuildDir 'logger-child.exe'

# cl.exe is only on PATH in a developer prompt; import that environment.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vs = if (Test-Path $vswhere) {
    & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
}
if (-not $vs) { throw 'need Visual Studio Build Tools with the "Desktop development with C++" workload' }
cmd /c "`"$vs\Common7\Tools\VsDevCmd.bat`" -arch=amd64 -no_logo && set" | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') { Set-Item "Env:$($matches[1])" $matches[2] -ErrorAction SilentlyContinue }
}

cl.exe /nologo /std:c++17 /EHsc /W4 /O2 /utf-8 "/Fo:$BuildDir\" "/Fe:$exe" (Join-Path $ChildDir 'src\main.cpp')
Assert-LastExitCode 'child build'

# Program Files, not the build directory: a LocalSystem service must not run
# from a location non-administrators can write.
Write-Host "`n-- installing to $InstallDir"
New-Item -ItemType Directory -Force -Path $InstallDir, $DataDir | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'agent-rust\target\release\flamingo-agent.exe') $InstallDir -Force
Copy-Item $exe $InstallDir -Force

# Ignore the failure when nothing is registered yet. PowerShell 7.4 turns a
# native non-zero exit into a terminating error, hence the try.
try { & $Agent --uninstall 2>$null } catch { }
& $Agent --install --data-dir $DataDir
Assert-LastExitCode 'install'

Start-Service $ServiceName
Get-Service $ServiceName | Format-Table -AutoSize Name, Status, StartType
Write-Host "agent log: $DataDir\agent.log"
Write-Host "child log: $DataDir\child.log (admins and SYSTEM only)"
