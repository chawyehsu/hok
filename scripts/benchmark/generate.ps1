#Requires -Version 5.1

<#
.SYNOPSIS
    Helper script to generate benchmark sandbox configuration.
.DESCRIPTION
    This script is used to generate benchmark scripts.
    It will replace the placeholder in the template files.
.PARAMETER MemoryLimit
    Specifies the memory limit for the sandbox.
    If not specified, the default value is 2048.
.PARAMETER Proxy
    Specifies the proxy server for the sandbox.
    If not specified, the default value is empty.
#>
param(
    [Parameter(Mandatory = $False)]
    [UInt32] $MemoryLimit = 2048,
    [Parameter(Mandatory = $False)]
    [Uri] $Proxy = ''
)

Set-StrictMode -Version Latest

function Invoke-ScriptGeneration {
    $path = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($PSScriptRoot)

    $startPs1 = (Get-Content -Path "$PSScriptRoot\bench.ps1.template" -Raw) -replace '{{proxy}}', $Proxy
    $sandboxWsb = (Get-Content -Path "$PSScriptRoot\sandbox.wsb.template" -Raw) -replace '{{memory}}', $MemoryLimit -replace '{{host}}', $path

    $startPs1 | Out-File -FilePath "$PSScriptRoot\bench.ps1" -Encoding utf8 -Force
    $sandboxWsb | Out-File -FilePath "$PSScriptRoot\sandbox.wsb" -Encoding utf8 -Force

    Write-Host 'SandBox configuration generated.' -ForegroundColor DarkGreen
    Write-Host 'Make sure you have enabled Windows Sandbox feature.'
    Write-Host "You can start the benchmark sandbox by running the 'sandbox.wsb' file:"
    Write-Host "$path\sandbox.wsb" -ForegroundColor DarkYellow
}

$oldErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Stop'

Invoke-ScriptGeneration

$ErrorActionPreference = $oldErrorActionPreference
