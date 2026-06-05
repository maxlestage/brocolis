# Brocolis - installation systeme sur Windows.
#
# `brocolis auto` est la voie native sur Windows (cf. src/system/windows.rs) :
# il fait exactement la meme chose en interactif et tout restaure a l'arret
# (Ctrl-C). Ce script est l'equivalent persistant - utile pour controler
# chaque etape, ou pour preparer le poste sans demarrer le proxy tout de
# suite.
#
# Setup automatique :
#   - dossier %ProgramData%\Brocolis\ ;
#   - CA + cle + brocolis.toml par defaut (si absents) ;
#   - approbation du CA dans le magasin Local Machine\Trusted Root via
#     certutil ;
#   - politique entreprise Firefox dans chaque variante detectee
#     (Mozilla Firefox, Developer Edition, Nightly) ;
#   - etat persiste dans %ProgramData%\Brocolis\state.json.
#
# Flag optionnel :
#   -SystemProxy        Regle le proxy WinHTTP via `netsh winhttp set proxy`
#                       (etat initial sauvegarde dans state.json pour
#                       restauration par uninstall.ps1).
#
# Variables d'environnement :
#   $env:BROCOLIS_PORT  Port d'ecoute (defaut 8080).
#
# Idempotent : re-executable sans dommage. L'etat (ce qui a ete cree par
# nous) est suivi dans state.json pour que uninstall.ps1 ne retire que ce
# qui doit l'etre.
#
# Usage :
#   PS> .\windows\install.ps1 [-SystemProxy]

[CmdletBinding()]
param(
    [switch] $SystemProxy,
    [switch] $Help
)

$ErrorActionPreference = 'Stop'

if ($Help) {
    Get-Content -LiteralPath $MyInvocation.MyCommand.Path -TotalCount 30 |
        ForEach-Object { $_ -replace '^# ?', '' }
    exit 0
}

# Constantes.
$ETC_DIR      = Join-Path $env:ProgramData 'Brocolis'
$STATE_FILE   = Join-Path $ETC_DIR 'state.json'
$CA_CRT       = Join-Path $ETC_DIR 'brocolis-ca.crt'
$CA_KEY       = Join-Path $ETC_DIR 'brocolis-ca.key'
$CONFIG_FILE  = Join-Path $ETC_DIR 'brocolis.toml'
$CA_CN        = 'Brocolis Root CA'
$LISTEN_PORT  = if ($env:BROCOLIS_PORT) { [int]$env:BROCOLIS_PORT } else { 8080 }

# Verifie elevation administrateur ; sinon, relance UAC.
$principal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host '==> Elevation des privileges (UAC) pour l''installation systeme...'
    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $MyInvocation.MyCommand.Path)
    if ($SystemProxy) { $argList += '-SystemProxy' }
    Start-Process -FilePath 'powershell.exe' -ArgumentList $argList -Verb RunAs
    exit
}

# 1. Localise le binaire brocolis (PATH d'abord, sinon target\release\).
$bin = $null
$cmd = Get-Command brocolis -ErrorAction SilentlyContinue
if ($cmd) {
    $bin = $cmd.Source
} else {
    $candidate = Join-Path $PSScriptRoot '..\target\release\brocolis.exe'
    if (Test-Path -LiteralPath $candidate) {
        $bin = (Resolve-Path -LiteralPath $candidate).Path
    }
}
if (-not $bin) {
    Write-Error "brocolis.exe introuvable (ni dans PATH, ni dans target\release\). Compilez d'abord : cargo build --release --bin brocolis"
    exit 2
}
Write-Host "Binaire utilise : $bin"

# 2. Cree le dossier de travail.
New-Item -ItemType Directory -Path $ETC_DIR -Force | Out-Null

# 3. Recharge l'etat precedent pour preserver les flags `INSTALLED_*=1` si
#    la ressource correspondante existe toujours.
$old = @{ INSTALLED_CA = 0; INSTALLED_FF_POLICIES = @(); INSTALLED_SYSTEM_PROXY = 0;
          INSTALLED_CONFIG = 0; SAVED_WINHTTP = $null }
if (Test-Path -LiteralPath $STATE_FILE) {
    try {
        $loaded = Get-Content -LiteralPath $STATE_FILE -Raw | ConvertFrom-Json
        foreach ($k in 'INSTALLED_CA','INSTALLED_FF_POLICIES','INSTALLED_SYSTEM_PROXY','INSTALLED_CONFIG','SAVED_WINHTTP') {
            if ($null -ne $loaded.$k) { $old[$k] = $loaded.$k }
        }
    } catch { }
}

# 4. Genere (ou recharge) le CA.
if (-not ((Test-Path -LiteralPath $CA_CRT) -and (Test-Path -LiteralPath $CA_KEY))) {
    Write-Host "==> Generation du CA dans $ETC_DIR..."
    & $bin ca --ca-cert $CA_CRT --ca-key $CA_KEY | Out-Null
} else {
    Write-Host "CA deja present dans $ETC_DIR : conserve."
}

# 5. Approuve le CA dans le magasin Trusted Root.
$installedCa = 0
$found = (& certutil.exe -store Root $CA_CN 2>&1) -join "`n"
if ($LASTEXITCODE -eq 0 -and $found -match [regex]::Escape($CA_CN)) {
    $installedCa = [int]$old.INSTALLED_CA
    Write-Host "CA deja present dans le magasin Trusted Root : conserve a l'arret."
} else {
    & certutil.exe -addstore -f Root $CA_CRT | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Error 'certutil -addstore Root a echoue.'
        exit 1
    }
    $installedCa = 1
    Write-Host "==> CA approuve dans Local Machine\Trusted Root Certification Authorities"
}

# 6. Politique entreprise Firefox.
$ffVariants = @(
    (Join-Path $env:ProgramFiles 'Mozilla Firefox'),
    (Join-Path ${env:ProgramFiles(x86)} 'Mozilla Firefox'),
    (Join-Path $env:ProgramFiles 'Firefox Developer Edition'),
    (Join-Path $env:ProgramFiles 'Firefox Nightly')
) | Sort-Object -Unique

$ffPolicyJson = @'
{
  "policies": {
    "Certificates": {
      "ImportEnterpriseRoots": true
    }
  }
}
'@

$installedFfPolicies = @()
$foundAny = $false
foreach ($app in $ffVariants) {
    if (-not (Test-Path -LiteralPath $app)) { continue }
    $foundAny = $true
    $dist   = Join-Path $app 'distribution'
    $policy = Join-Path $dist 'policies.json'
    if (Test-Path -LiteralPath $policy) {
        Write-Host "Politique Firefox deja presente ($policy) : laissee intacte."
        # Si on l'avait deja installee selon l'ancien state, on la conserve.
        if ($old.INSTALLED_FF_POLICIES -contains $policy) {
            $installedFfPolicies += $policy
        }
        continue
    }
    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    Set-Content -LiteralPath $policy -Value $ffPolicyJson -Encoding UTF8
    $installedFfPolicies += $policy
    Write-Host "==> Politique Firefox deposee ($policy)"
}
if (-not $foundAny) {
    Write-Host 'Aucun Firefox detecte dans Program Files - etape ignoree.'
}

# 7. Genere un brocolis.toml par defaut.
$installedConfig = 0
if (-not (Test-Path -LiteralPath $CONFIG_FILE)) {
    $key = ((& $bin keygen) | Select-Object -First 1).Trim()
    $caEsc  = $CA_CRT.Replace('\','\\')
    $keyEsc = $CA_KEY.Replace('\','\\')
    @"
listen = "127.0.0.1:$LISTEN_PORT"
ca_cert = "$caEsc"
ca_key = "$keyEsc"
key = "$key"
encrypt_hosts = []
"@ | Set-Content -LiteralPath $CONFIG_FILE -Encoding UTF8
    $installedConfig = 1
    Write-Host "==> Config generee : $CONFIG_FILE (port $LISTEN_PORT)"
} else {
    $installedConfig = [int]$old.INSTALLED_CONFIG
    Write-Host "Config deja presente : $CONFIG_FILE - non ecrasee."
}

# 8. Proxy WinHTTP systeme (optionnel).
$installedSystemProxy = 0
$savedWinHttp = $old.SAVED_WINHTTP
if ($SystemProxy) {
    # Sauvegarde l'etat actuel pour restauration.
    if (-not $savedWinHttp) {
        $show = ((& netsh.exe winhttp show proxy) -join "`n")
        $direct = ($show -match '(?i)direct access')
        $server = ''
        $bypass = ''
        if ($show -match 'Proxy Server\(s\)\s*:\s*(\S+)') { $server = $Matches[1] }
        if ($show -match 'Bypass List\s*:\s*(.+)')        { $bypass = $Matches[1].Trim() }
        $savedWinHttp = @{ was_direct = [bool]$direct; server = $server; bypass = $bypass }
    }
    & netsh.exe winhttp set proxy proxy-server="127.0.0.1:$LISTEN_PORT" bypass-list='<local>' | Out-Null
    $installedSystemProxy = 1
    Write-Host "==> Proxy WinHTTP regle sur 127.0.0.1:$LISTEN_PORT"
}

# 9. Persiste l'etat.
$state = [ordered]@{
    INSTALLED_CA            = $installedCa
    INSTALLED_FF_POLICIES   = $installedFfPolicies
    INSTALLED_CONFIG        = $installedConfig
    INSTALLED_SYSTEM_PROXY  = $installedSystemProxy
    SAVED_WINHTTP           = $savedWinHttp
    ETC_DIR                 = $ETC_DIR
    CA_CRT                  = $CA_CRT
    CA_KEY                  = $CA_KEY
    CONFIG_FILE             = $CONFIG_FILE
    CA_CN                   = $CA_CN
}
$state | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $STATE_FILE -Encoding UTF8

Write-Host ''
Write-Host 'Installation terminee.'
Write-Host ''
Write-Host 'Demarrer le proxy :'
Write-Host "    brocolis run --config `"$CONFIG_FILE`""
Write-Host ''
Write-Host "Configurez votre navigateur sur le proxy HTTP/HTTPS 127.0.0.1:$LISTEN_PORT"
Write-Host '(Firefox suivra la politique entreprise deposee).'
Write-Host ''
Write-Host 'Desinstaller :'
Write-Host "    .\windows\uninstall.ps1"
