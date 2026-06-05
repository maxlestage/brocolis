# Test d'integration ciblé sur `brocolis auto` (chemin natif Windows).
#
# Couvre, sur une vraie machine Windows (admin requis) :
#   - le démarrage de auto et la pose des artefacts (CA dans le magasin
#     Local Machine\Trusted Root, politique entreprise Firefox dans
#     chaque variante détectée, proxy WinHTTP via netsh) ;
#   - un vrai flux HTTPS via le proxy (CONNECT + cert forgé) avec --cacert
#     explicite ET via la confiance transparente du magasin système ;
#   - le round-trip du chiffrement sélectif contre une origine d'écho
#     Python locale ;
#   - la restauration à l'arrêt (Ctrl+C envoyé proprement via Win32 API)
#     : CA retiré, politiques Firefox retirées, WinHTTP restauré tel quel.
#
# Complémentaire de windows/install.ps1 (chemin persistant). Equivalent
# direct de linux/test-auto.sh.
#
# Usage (depuis la racine du dépôt, terminal admin) :
#   PS> .\windows\test-auto.ps1
#
# Le script compile la CLI (cargo build --release --bin brocolis), puis
# joue le cycle complet. Il restaure l'état initial en sortant.

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
$ROOT = Split-Path $PSScriptRoot -Parent
$BIN  = Join-Path $ROOT 'target\release\brocolis.exe'
if (-not (Test-Path -LiteralPath $BIN)) {
    Write-Host '==> Compilation (cargo build --release --bin brocolis)...'
    Push-Location $ROOT
    try { & cargo build --release --bin brocolis } finally { Pop-Location }
    if ($LASTEXITCODE -ne 0) { exit 1 }
}
if (-not (Test-Path -LiteralPath $BIN)) {
    Write-Error "Binaire absent après compilation : $BIN"
    exit 2
}

$PORT       = 8893
$ECHO_PORT  = 8894
$CA_CN      = 'Brocolis Root CA'

# Variantes Firefox à scanner (miroir de src/system/windows.rs).
$FF_VARIANTS = @(
    (Join-Path $env:ProgramFiles 'Mozilla Firefox'),
    (Join-Path ${env:ProgramFiles(x86)} 'Mozilla Firefox'),
    (Join-Path $env:ProgramFiles 'Firefox Developer Edition'),
    (Join-Path $env:ProgramFiles 'Firefox Nightly')
) | Sort-Object -Unique

# Compteurs.
$script:PASS = 0
$script:FAIL = 0
function Ok([string]$msg) { Write-Host "  PASS: $msg"; $script:PASS++ }
function Ko([string]$msg) { Write-Host "  FAIL: $msg"; $script:FAIL++ }
function Info([string]$msg) { Write-Host "  INFO: $msg" }

# Helpers de détection.
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
function Ff-Policy-Existing {
    foreach ($app in $FF_VARIANTS) {
        if (-not (Test-Path -LiteralPath $app)) { continue }
        $policy = Join-Path $app 'distribution\policies.json'
        if (Test-Path -LiteralPath $policy) { return $true }
    }
    return $false
}

# Etat initial.
$CA_PRE  = Ca-Present
$FF_PRE  = Ff-Policy-Existing
$WH_PRE  = Read-WinHttp

# Définit Send-CtrlC : envoie un vrai CTRL_C_EVENT au processus visé via
# AttachConsole + GenerateConsoleCtrlEvent. Nécessaire pour que tokio::signal
# côté brocolis déclenche revert() proprement.
$null = Add-Type -Namespace BrocolisTest -Name Win32 -PassThru -MemberDefinition @'
[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError=true)]
public static extern bool AttachConsole(uint dwProcessId);

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError=true)]
public static extern bool FreeConsole();

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError=true)]
public static extern bool GenerateConsoleCtrlEvent(uint dwCtrlEvent, uint dwProcessGroupId);

[System.Runtime.InteropServices.DllImport("kernel32.dll", SetLastError=true)]
public static extern bool SetConsoleCtrlHandler(System.IntPtr HandlerRoutine, bool Add);
'@

function Send-CtrlC {
    param([int]$ProcessId)
    # On se détache de notre console, on s'attache à celle du fils, on ignore
    # nos propres Ctrl+C, on envoie l'évènement à tout le groupe (0), on
    # restaure la situation. Sans `SetConsoleCtrlHandler(NULL, TRUE)` on
    # se tuerait nous-mêmes.
    $sent = $false
    try {
        [BrocolisTest.Win32]::FreeConsole() | Out-Null
        if ([BrocolisTest.Win32]::AttachConsole([uint32]$ProcessId)) {
            [BrocolisTest.Win32]::SetConsoleCtrlHandler([System.IntPtr]::Zero, $true) | Out-Null
            $sent = [BrocolisTest.Win32]::GenerateConsoleCtrlEvent(0, 0)
            Start-Sleep -Seconds 1
        }
    }
    finally {
        [BrocolisTest.Win32]::FreeConsole() | Out-Null
        [BrocolisTest.Win32]::SetConsoleCtrlHandler([System.IntPtr]::Zero, $false) | Out-Null
    }
    return $sent
}

# Espace de travail.
$WORK = Join-Path ([System.IO.Path]::GetTempPath()) ("brocolis-auto-test-" + [Guid]::NewGuid().ToString().Substring(0,8))
New-Item -ItemType Directory -Path $WORK | Out-Null

$brocolisProc = $null
$echoProc = $null

try {
    Push-Location $WORK

    # Génère CA + config dans le workdir.
    $caCrt = Join-Path $WORK 'brocolis-ca.crt'
    $caKey = Join-Path $WORK 'brocolis-ca.key'
    & $BIN ca --ca-cert $caCrt --ca-key $caKey | Out-Null
    $key = ((& $BIN keygen) | Select-Object -First 1).Trim()
    $caEsc  = $caCrt.Replace('\','\\')
    $keyEsc = $caKey.Replace('\','\\')
    $cfg = @"
listen = "127.0.0.1:$PORT"
ca_cert = "$caEsc"
ca_key = "$keyEsc"
key = "$key"
encrypt_hosts = []
"@
    $cfg | Set-Content -LiteralPath 'brocolis.toml' -Encoding UTF8

    # Origine d'écho locale (round-trip chiffrement), si Python dispo.
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

    Write-Host "==> Démarrage de « brocolis auto --config $WORK\brocolis.toml »..."
    $env:RUST_LOG = 'info'
    $brocolisProc = Start-Process -FilePath $BIN `
                        -ArgumentList @('auto', '--config', 'brocolis.toml') `
                        -RedirectStandardOutput 'auto.log' `
                        -RedirectStandardError  'auto.err' `
                        -PassThru -NoNewWindow

    # Attend "écoute" dans les logs.
    $started = $false
    for ($i=0; $i -lt 80; $i++) {
        if ($brocolisProc.HasExited) { break }
        $stdout = if (Test-Path 'auto.log') { Get-Content 'auto.log' -Raw -ErrorAction SilentlyContinue } else { '' }
        $stderr = if (Test-Path 'auto.err') { Get-Content 'auto.err' -Raw -ErrorAction SilentlyContinue } else { '' }
        if ($stdout -match 'écoute' -or $stderr -match 'écoute') { $started = $true; break }
        Start-Sleep -Milliseconds 200
    }
    if ($started) { Ok 'proxy démarré' } else {
        Ko 'proxy non démarré'
        if (Test-Path 'auto.err') { Get-Content 'auto.err' | Select-Object -First 40 | Out-Host }
        if (Test-Path 'auto.log') { Get-Content 'auto.log' | Select-Object -First 40 | Out-Host }
        exit 1
    }

    Write-Host '== Pendant l''exécution =='

    # CA dans le magasin Trusted Root.
    if (Ca-Present) { Ok 'CA présent dans Local Machine\Trusted Root' } else { Ko 'CA absent du magasin Trusted Root' }

    # Politique Firefox dans au moins une variante détectée.
    $ffOk = $false; $ffSeen = 0
    foreach ($app in $FF_VARIANTS) {
        if (-not (Test-Path -LiteralPath $app)) { continue }
        $ffSeen++
        $policy = Join-Path $app 'distribution\policies.json'
        if ((Test-Path -LiteralPath $policy) -and ((Get-Content $policy -Raw) -match 'ImportEnterpriseRoots')) {
            Ok "politique Firefox déposée ($policy)"
            $ffOk = $true
        }
    }
    if ($ffSeen -eq 0) {
        Info 'aucun Firefox installé - étape Firefox ignorée'
    } elseif (-not $ffOk -and -not $FF_PRE) {
        Ko 'politique Firefox manquante dans toutes les variantes'
    }

    # Proxy WinHTTP réglé sur 127.0.0.1:$PORT.
    $whNow = Read-WinHttp
    if (-not $whNow.Direct -and $whNow.Server -match "127\.0\.0\.1:$PORT") {
        Ok 'proxy WinHTTP réglé sur 127.0.0.1:' + $PORT
    } else {
        Ko 'proxy WinHTTP non réglé correctement'
        Write-Host $whNow.Raw
    }

    # Flux HTTPS réel via le proxy (curl.exe est inclus dans Windows 10+).
    $haveCurl = $null -ne (Get-Command curl.exe -ErrorAction SilentlyContinue)
    if ($haveCurl) {
        $code = & curl.exe -s -o ex.html -w '%{http_code}' `
                    -x "http://127.0.0.1:$PORT" --cacert $caCrt `
                    'https://example.com/' 2>$null
        $body = if (Test-Path 'ex.html') { Get-Content 'ex.html' -Raw } else { '' }
        if ($code -eq '200' -and $body -match '(?i)Example Domain') {
            Ok 'MITM HTTPS réel (example.com → 200)'
        } else {
            Ko "MITM HTTPS réel échoué (code=$code)"
        }

        # Confiance transparente via le magasin système.
        $tcode = & curl.exe -s -o NUL -w '%{http_code}' `
                    -x "http://127.0.0.1:$PORT" 'https://example.com/' 2>$null
        if ($tcode -eq '200') {
            Ok 'curl fait confiance au CA via le magasin système (transparent)'
        } else {
            Info "curl backend ne lit pas le magasin système (code=$tcode) - non bloquant"
        }
    } else {
        Info 'curl.exe absent - flux HTTPS ignoré'
    }

    # Round-trip chiffrement sélectif.
    if ($havePython -and $haveCurl) {
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

    Write-Host '== Arrêt et vérification de la restauration =='
    $sent = Send-CtrlC -ProcessId $brocolisProc.Id
    if (-not $sent) {
        Info "Send-CtrlC a échoué - fallback Stop-Process (revert peut ne pas tourner)"
        Stop-Process -Id $brocolisProc.Id -Force -ErrorAction SilentlyContinue
    }

    # Attend la sortie propre.
    for ($i=0; $i -lt 60; $i++) {
        if ($brocolisProc.HasExited) { break }
        Start-Sleep -Milliseconds 200
    }
    if (-not $brocolisProc.HasExited) {
        Ko "le processus ne s'est pas arrêté proprement"
        Stop-Process -Id $brocolisProc.Id -Force -ErrorAction SilentlyContinue
    }

    # Revert : CA retiré (si CA_PRE=false) ou conservé (si CA_PRE=true).
    $caNow = Ca-Present
    if (-not $CA_PRE) {
        if (-not $caNow) { Ok 'CA retiré du magasin (état initial restauré)' }
        else             { Ko 'CA non retiré du magasin' }
    } else {
        if ($caNow) { Ok 'CA préexistant conservé (comportement correct)' }
        else        { Ko 'CA préexistant supprimé à tort' }
    }

    # Politique Firefox : retirée seulement dans les variantes qu'on a touchées.
    if (-not $FF_PRE -and $ffSeen -gt 0) {
        $anyLeft = $false
        foreach ($app in $FF_VARIANTS) {
            if (-not (Test-Path -LiteralPath $app)) { continue }
            $policy = Join-Path $app 'distribution\policies.json'
            if (Test-Path -LiteralPath $policy) { $anyLeft = $true; Write-Host "    reste : $policy" }
        }
        if (-not $anyLeft) { Ok 'politique(s) Firefox retirée(s)' } else { Ko 'politique Firefox subsiste' }
    } elseif ($FF_PRE) {
        Info 'politique Firefox préexistait - vérif de conservation ignorée (manuelle)'
    }

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
    Pop-Location -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $WORK -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ''
Write-Host '=================================================='
Write-Host ("  Résultat : {0} réussi(s), {1} échoué(s)" -f $script:PASS, $script:FAIL)
Write-Host '=================================================='
if ($script:FAIL -eq 0) { exit 0 } else { exit 1 }
