# Test d'intégration Windows pour le chemin install.ps1 / uninstall.ps1.
#
# Couvre, sur une vraie machine Windows admin :
#   - install.ps1 -SystemProxy pose CA dans Local Machine\Trusted Root ;
#   - politique entreprise Firefox dans chaque variante détectée ;
#   - brocolis.toml généré dans %ProgramData%\Brocolis\ ;
#   - proxy WinHTTP réglé via netsh ;
#   - vrai flux HTTPS via le proxy lancé manuellement (CONNECT + cert
#     forgé) avec --cacert et via la confiance transparente du magasin ;
#   - chiffrement sélectif round-trip ;
#   - uninstall.ps1 restaure tout (CA, politiques Firefox, brocolis.toml,
#     WinHTTP à son état initial), en respectant le préexistant.
#
# Complémentaire de windows/test-auto.ps1, qui teste le chemin natif
# `brocolis auto`.
#
# Usage (depuis la racine du dépôt, terminal admin) :
#   PS> .\windows\test-install.ps1

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# Élévation requise (pas d'auto-UAC pour ne pas perdre stdout / exit code).
$principal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "Lancez ce script depuis un terminal 'Exécuter en tant qu'administrateur'."
    exit 2
}

# Localise / compile le binaire.
$ROOT       = Split-Path $PSScriptRoot -Parent
$BIN        = Join-Path $ROOT 'target\release\brocolis.exe'
$INSTALL    = Join-Path $PSScriptRoot 'install.ps1'
$UNINSTALL  = Join-Path $PSScriptRoot 'uninstall.ps1'
if (-not (Test-Path -LiteralPath $BIN)) {
    Write-Host '==> Compilation (cargo build --release --bin brocolis)...'
    Push-Location $ROOT
    try { & cargo build --release --bin brocolis } finally { Pop-Location }
    if ($LASTEXITCODE -ne 0) { exit 1 }
}
foreach ($p in @($BIN, $INSTALL, $UNINSTALL)) {
    if (-not (Test-Path -LiteralPath $p)) { Write-Error "Manquant : $p"; exit 2 }
}

$PORT       = 8892
$ECHO_PORT  = 8891
$CA_CN      = 'Brocolis Root CA'
$ETC        = Join-Path $env:ProgramData 'Brocolis'
$STATE_FILE = Join-Path $ETC 'state.json'
$CA_CRT     = Join-Path $ETC 'brocolis-ca.crt'
$CONFIG     = Join-Path $ETC 'brocolis.toml'

$FF_VARIANTS = @(
    (Join-Path $env:ProgramFiles 'Mozilla Firefox'),
    (Join-Path ${env:ProgramFiles(x86)} 'Mozilla Firefox'),
    (Join-Path $env:ProgramFiles 'Firefox Developer Edition'),
    (Join-Path $env:ProgramFiles 'Firefox Nightly')
) | Sort-Object -Unique

$script:PASS = 0
$script:FAIL = 0
function Ok([string]$msg)   { Write-Host "  PASS: $msg"; $script:PASS++ }
function Ko([string]$msg)   { Write-Host "  FAIL: $msg"; $script:FAIL++ }
function Info([string]$msg) { Write-Host "  INFO: $msg" }

function Ca-Present {
    $out = (& certutil.exe -store Root $CA_CN 2>&1) -join "`n"
    return ($LASTEXITCODE -eq 0 -and $out -match [regex]::Escape($CA_CN))
}
function Read-WinHttp {
    $out = ((& netsh.exe winhttp show proxy) -join "`n")
    $direct = ($out -match '(?i)direct access')
    $server = ''
    if ($out -match 'Proxy Server\(s\)\s*:\s*(\S+)') { $server = $Matches[1] }
    return @{ Direct = [bool]$direct; Server = $server; Raw = $out }
}

# État initial à restaurer/vérifier.
$CA_PRE  = Ca-Present
$WH_PRE  = Read-WinHttp
$CFG_PRE = Test-Path -LiteralPath $CONFIG
$FF_PRE  = @{}
foreach ($app in $FF_VARIANTS) {
    $FF_PRE[$app] = Test-Path -LiteralPath (Join-Path $app 'distribution\policies.json')
}

$WORK = Join-Path ([System.IO.Path]::GetTempPath()) ("brocolis-install-test-" + [Guid]::NewGuid().ToString().Substring(0,8))
New-Item -ItemType Directory -Path $WORK | Out-Null

$brocolisProc = $null
$echoProc = $null
$ranInstall = $false

try {
    Push-Location $WORK

    # Origine d'écho locale, si Python dispo.
    $havePython = $null -ne (Get-Command python -ErrorAction SilentlyContinue)
    if ($havePython) {
        @'
import sys
from http.server import BaseHTTPRequestHandler, HTTPServer
PORT = int(sys.argv[1])
class E(BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get('Content-Length', 0))
        b = self.rfile.read(n)
        sys.stderr.write("ORIGIN:" + b.decode('utf-8', 'replace') + "\n")
        sys.stderr.flush()
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain')
        self.send_header('Content-Length', str(len(b)))
        self.end_headers()
        self.wfile.write(b)
    def log_message(self, *a):
        pass
HTTPServer(('127.0.0.1', PORT), E).serve_forever()
'@ | Set-Content -LiteralPath 'echo.py' -Encoding UTF8
        $echoProc = Start-Process -FilePath python -ArgumentList @('echo.py', "$ECHO_PORT") `
                        -RedirectStandardError 'echo.log' -PassThru -NoNewWindow
    } else {
        Info 'python absent - round-trip de chiffrement sera ignoré'
    }

    Write-Host "==> Exécution de install.ps1 -SystemProxy (BROCOLIS_PORT=$PORT)..."
    $env:BROCOLIS_PORT = "$PORT"
    & $INSTALL -SystemProxy *> install.log
    $rc = $LASTEXITCODE
    if ($rc -ne 0) {
        Ko "install.ps1 a échoué (rc=$rc)"
        Get-Content 'install.log' -ErrorAction SilentlyContinue | Out-Host
        exit 1
    }
    $ranInstall = $true

    Write-Host '== Pendant l''exécution =='

    if (Ca-Present)                      { Ok 'CA présent dans Local Machine\Trusted Root' }
    else                                 { Ko 'CA absent du magasin Trusted Root' }

    $ffSeen = 0
    foreach ($app in $FF_VARIANTS) {
        if (-not (Test-Path -LiteralPath $app)) { continue }
        $ffSeen++
        $policy = Join-Path $app 'distribution\policies.json'
        if ((Test-Path -LiteralPath $policy) -and ((Get-Content $policy -Raw) -match 'ImportEnterpriseRoots')) {
            Ok "politique Firefox déposée ($policy)"
        } elseif ($FF_PRE[$app]) {
            Info "politique préexistait dans $app - non écrasée"
        } else {
            Ko "politique Firefox manquante dans $app"
        }
    }
    if ($ffSeen -eq 0) { Info 'aucun Firefox installé - étape Firefox ignorée' }

    if (Test-Path -LiteralPath $CONFIG) { Ok "brocolis.toml présent ($CONFIG)" }
    else                                { Ko 'brocolis.toml absent' }

    $whNow = Read-WinHttp
    if (-not $whNow.Direct -and $whNow.Server -match "127\.0\.0\.1:$PORT") {
        Ok "proxy WinHTTP réglé sur 127.0.0.1:$PORT"
    } else {
        Ko 'proxy WinHTTP non réglé correctement'
        Write-Host $whNow.Raw
    }

    Write-Host "==> Démarrage de « brocolis run --config $CONFIG »..."
    $env:RUST_LOG = 'info'
    $brocolisProc = Start-Process -FilePath $BIN `
                        -ArgumentList @('run', '--config', $CONFIG) `
                        -RedirectStandardOutput 'auto.log' `
                        -RedirectStandardError  'auto.err' `
                        -PassThru -NoNewWindow

    $started = $false
    for ($i=0; $i -lt 80; $i++) {
        if ($brocolisProc.HasExited) { break }
        $log = if (Test-Path 'auto.log') { Get-Content 'auto.log' -Raw -ErrorAction SilentlyContinue } else { '' }
        $err = if (Test-Path 'auto.err') { Get-Content 'auto.err' -Raw -ErrorAction SilentlyContinue } else { '' }
        if ($log -match 'écoute' -or $err -match 'écoute') { $started = $true; break }
        Start-Sleep -Milliseconds 200
    }
    if ($started) { Ok 'proxy démarré' } else {
        Ko 'proxy non démarré'
        if (Test-Path 'auto.err') { Get-Content 'auto.err' | Select-Object -First 40 | Out-Host }
        exit 1
    }

    # Flux HTTPS via le proxy.
    if ($null -ne (Get-Command curl.exe -ErrorAction SilentlyContinue)) {
        $code = & curl.exe -s -o ex.html -w '%{http_code}' `
                    -x "http://127.0.0.1:$PORT" --cacert $CA_CRT `
                    'https://example.com/' 2>$null
        $body = if (Test-Path 'ex.html') { Get-Content 'ex.html' -Raw } else { '' }
        if ($code -eq '200' -and $body -match '(?i)Example Domain') {
            Ok 'MITM HTTPS réel (example.com → 200)'
        } else {
            Ko "MITM HTTPS réel échoué (code=$code)"
        }

        $tcode = & curl.exe -s -o NUL -w '%{http_code}' `
                    -x "http://127.0.0.1:$PORT" 'https://example.com/' 2>$null
        if ($tcode -eq '200') {
            Ok 'curl fait confiance au CA via le magasin système (transparent)'
        } else {
            Info "curl backend ne lit pas le magasin système (code=$tcode) - non bloquant"
        }

        if ($havePython) {
            $out = & curl.exe -s -x "http://127.0.0.1:$PORT" `
                        "http://127.0.0.1:$ECHO_PORT/" `
                        --data-binary 'a ((secret)) b' `
                        -H 'Content-Type: text/plain' 2>$null
            $orig = if (Test-Path 'echo.log') { (Get-Content 'echo.log' -Raw) } else { '' }
            if ($out -eq 'a secret b' -and $orig -match 'BROCOLIS:v1:') {
                Ok 'chiffrement sélectif round-trip (origine ne voit que le chiffré)'
            } else {
                Ko 'chiffrement sélectif round-trip'
                Write-Host "  reçu='$out' origine='$orig'"
            }
        }
    } else {
        Info 'curl.exe absent - flux HTTPS ignoré'
    }

    Write-Host '== Arrêt du proxy puis désinstallation =='
    Stop-Process -Id $brocolisProc.Id -Force -ErrorAction SilentlyContinue
    for ($i=0; $i -lt 30; $i++) {
        if ($brocolisProc.HasExited) { break }
        Start-Sleep -Milliseconds 200
    }

    & $UNINSTALL *> uninstall.log
    $rc = $LASTEXITCODE
    if ($rc -ne 0) {
        Ko "uninstall.ps1 a échoué (rc=$rc)"
        Get-Content 'uninstall.log' -ErrorAction SilentlyContinue | Out-Host
    }
    $ranInstall = $false  # uninstall a tourné, plus besoin du fallback du finally

    # Vérification de la restauration.
    $caNow = Ca-Present
    if (-not $CA_PRE) {
        if (-not $caNow) { Ok 'CA retiré du magasin (état initial restauré)' }
        else             { Ko 'CA non retiré du magasin' }
    } else {
        if ($caNow) { Ok 'CA préexistant conservé' }
        else        { Ko 'CA préexistant supprimé à tort' }
    }

    foreach ($app in $FF_VARIANTS) {
        if (-not (Test-Path -LiteralPath $app)) { continue }
        $policy = Join-Path $app 'distribution\policies.json'
        if (-not $FF_PRE[$app]) {
            if (-not (Test-Path -LiteralPath $policy)) {
                Ok "politique Firefox retirée ($app)"
            } else {
                Ko "politique Firefox non retirée ($app)"
            }
        } else {
            if (Test-Path -LiteralPath $policy) {
                Ok "politique Firefox préexistante conservée ($app)"
            } else {
                Ko "politique Firefox préexistante supprimée à tort ($app)"
            }
        }
    }

    if (-not $CFG_PRE) {
        if (-not (Test-Path -LiteralPath $CONFIG)) { Ok 'brocolis.toml retiré' }
        else                                       { Ko 'brocolis.toml non retiré' }
    }

    if (Test-Path -LiteralPath $STATE_FILE) { Ko 'state.json non nettoyé' }
    else                                    { Ok 'state.json nettoyé' }

    # WinHTTP restauré tel quel.
    $whAfter = Read-WinHttp
    if ($WH_PRE.Direct -and $whAfter.Direct) {
        Ok 'WinHTTP restauré à direct (état initial)'
    } elseif (-not $WH_PRE.Direct -and -not $whAfter.Direct -and $whAfter.Server -eq $WH_PRE.Server) {
        Ok 'WinHTTP restauré au proxy initial'
    } else {
        Ko 'WinHTTP non restauré'
        Write-Host "  avant : $($WH_PRE.Raw)"
        Write-Host "  après : $($whAfter.Raw)"
    }
}
finally {
    if ($brocolisProc -and -not $brocolisProc.HasExited) {
        Stop-Process -Id $brocolisProc.Id -Force -ErrorAction SilentlyContinue
    }
    if ($echoProc -and -not $echoProc.HasExited) {
        Stop-Process -Id $echoProc.Id -Force -ErrorAction SilentlyContinue
    }
    # Garantit la restauration même en cas de crash entre install et uninstall.
    if ($ranInstall -and (Test-Path -LiteralPath $STATE_FILE)) {
        & $UNINSTALL *>$null
    }
    Pop-Location -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $WORK -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path Env:BROCOLIS_PORT -ErrorAction SilentlyContinue
}

Write-Host ''
Write-Host '=================================================='
Write-Host ("  Résultat : {0} réussi(s), {1} échoué(s)" -f $script:PASS, $script:FAIL)
Write-Host '=================================================='
if ($script:FAIL -eq 0) { exit 0 } else { exit 1 }
