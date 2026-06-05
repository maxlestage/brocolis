# Brocolis - desinstallation systeme sur Windows.
#
# Annule ce que install.ps1 a fait, en respectant ce qui preexistait
# (state.json tranche : seuls les `INSTALLED_*=1` sont retires).
#
# Ordre des operations :
#   1. restauration du proxy WinHTTP (si on l'avait change) ;
#   2. retrait du CA du magasin Trusted Root ;
#   3. retrait des politiques Firefox (celles qu'on a creees) ;
#   4. retrait de brocolis.toml ;
#   5. nettoyage de %ProgramData%\Brocolis\state.json.
#
# Le CA et la cle sous %ProgramData%\Brocolis\ ne sont PAS supprimes par
# defaut : ils peuvent resservir a une reinstallation. -Purge les efface
# aussi.
#
# Usage :
#   PS> .\windows\uninstall.ps1 [-Purge]

[CmdletBinding()]
param(
    [switch] $Purge,
    [switch] $Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    Get-Content -LiteralPath $MyInvocation.MyCommand.Path -TotalCount 20 |
        ForEach-Object { $_ -replace '^# ?', '' }
    exit 0
}

$ETC_DIR    = Join-Path $env:ProgramData 'Brocolis'
$STATE_FILE = Join-Path $ETC_DIR 'state.json'

# Elevation administrateur.
$principal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host '==> Elevation des privileges (UAC)...'
    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $MyInvocation.MyCommand.Path)
    if ($Purge) { $argList += '-Purge' }
    Start-Process -FilePath 'powershell.exe' -ArgumentList $argList -Verb RunAs
    exit
}

if (-not (Test-Path -LiteralPath $STATE_FILE)) {
    Write-Error "Aucun etat d'installation trouve ($STATE_FILE). Rien a desinstaller."
    exit 2
}

$state = Get-Content -LiteralPath $STATE_FILE -Raw | ConvertFrom-Json

# 1. Restauration du proxy WinHTTP.
if ([int]$state.INSTALLED_SYSTEM_PROXY -eq 1 -and $state.SAVED_WINHTTP) {
    $saved = $state.SAVED_WINHTTP
    if ($saved.was_direct) {
        & netsh.exe winhttp reset proxy | Out-Null
    } else {
        $bypassArg = if ([string]::IsNullOrEmpty($saved.bypass)) { 'bypass-list=' } else { "bypass-list=$($saved.bypass)" }
        & netsh.exe winhttp set proxy "proxy-server=$($saved.server)" $bypassArg | Out-Null
    }
    Write-Host 'OK Proxy WinHTTP restaure a son etat initial'
}

# 2. CA du magasin Trusted Root.
if ([int]$state.INSTALLED_CA -eq 1) {
    $caCn = if ($state.CA_CN) { $state.CA_CN } else { 'Brocolis Root CA' }
    $found = (& certutil.exe -store Root $caCn 2>&1) -join "`n"
    if ($LASTEXITCODE -eq 0 -and $found -match [regex]::Escape($caCn)) {
        & certutil.exe -delstore Root $caCn | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "OK CA retire du magasin Trusted Root ($caCn)"
        } else {
            Write-Warning 'certutil -delstore Root a echoue'
        }
    } else {
        Write-Host '  CA deja absent du magasin - rien a retirer'
    }
} else {
    Write-Host "  CA preexistant conserve (ce n'est pas nous qui l'avons pose)"
}

# 3. Politiques Firefox : on retire uniquement les fichiers qu'on a crees.
if ($state.INSTALLED_FF_POLICIES) {
    foreach ($policy in $state.INSTALLED_FF_POLICIES) {
        if (Test-Path -LiteralPath $policy) {
            Remove-Item -LiteralPath $policy -Force
            $dist = Split-Path -Parent $policy
            # Retire le dossier `distribution` s'il est devenu vide.
            if (Test-Path -LiteralPath $dist) {
                $children = Get-ChildItem -LiteralPath $dist -Force
                if (-not $children) { Remove-Item -LiteralPath $dist -Force }
            }
            Write-Host "OK Politique Firefox retiree ($policy)"
        }
    }
}

# 4. brocolis.toml.
if ([int]$state.INSTALLED_CONFIG -eq 1 -and (Test-Path -LiteralPath $state.CONFIG_FILE)) {
    Remove-Item -LiteralPath $state.CONFIG_FILE -Force
    Write-Host "OK brocolis.toml retire ($($state.CONFIG_FILE))"
}

# 5. Purge complete (CA + cle).
if ($Purge) {
    if (Test-Path -LiteralPath $state.ETC_DIR) {
        Remove-Item -LiteralPath $state.ETC_DIR -Recurse -Force
        Write-Host "OK $($state.ETC_DIR) supprime (CA + cle inclus) - -Purge"
    }
} else {
    Remove-Item -LiteralPath $STATE_FILE -Force
    # Retire le dossier s'il est devenu vide (sans -Purge, CA + cle restent).
    $items = Get-ChildItem -LiteralPath $ETC_DIR -Force -ErrorAction SilentlyContinue
    if (-not $items) { Remove-Item -LiteralPath $ETC_DIR -Force -ErrorAction SilentlyContinue }
}

Write-Host ''
Write-Host 'Desinstallation terminee.'
