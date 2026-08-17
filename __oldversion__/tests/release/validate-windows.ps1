param(
  [Parameter(Mandatory = $true)]
  [string]$OsName
)

$ErrorActionPreference = "Stop"
$EvidenceDirectory = Join-Path $PSScriptRoot "evidence"
New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null

$Installer = Get-ChildItem `
  -Path "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis" `
  -Filter "*.exe" |
  Select-Object -First 1

if (-not $Installer) {
  throw "No packaged NSIS installer was produced."
}

$Signature = Get-AuthenticodeSignature $Installer.FullName
if ($env:MTGO_NOTES_REQUIRE_PRODUCTION_SIGNATURE -eq "1" -and $Signature.Status -ne "Valid") {
  throw "Production release requires a valid Authenticode signature."
}

$WebView2 = Get-ItemProperty `
  "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\*" `
  -ErrorAction SilentlyContinue |
  Where-Object { $_.name -like "*WebView2*" } |
  Select-Object -First 1

$Os = Get-CimInstance Win32_OperatingSystem
$Evidence = [ordered]@{
  schemaVersion = 1
  requestedOs = $OsName
  osCaption = $Os.Caption
  osVersion = $Os.Version
  architecture = $env:PROCESSOR_ARCHITECTURE
  installer = $Installer.Name
  authenticode = $Signature.Status.ToString()
  webView2Present = [bool]$WebView2
  automatedChecks = @(
    "full-verify"
    "windows-target-tests"
    "x64-nsis-package"
    "classifier-signature-and-golden-vectors"
    "updater-trusted-and-tampered-fixtures"
    "offline-local-workflow-contracts"
    "diagnostic-canary-and-no-upload-contracts"
    "capability-policy"
  )
  manualEvidenceRequired = @(
    "real-mtgo-uia-and-ocr"
    "dpi-100-125-150-200"
    "multi-monitor-overlay-position"
    "non-activating-overlay"
    "tray-and-global-shortcut"
    "keyboard-only-and-screen-reader"
    "signed-production-updater-interruption-and-rollback"
    "capture-search-classification-and-startup-performance"
  )
  generatedAtUtc = [DateTime]::UtcNow.ToString("o")
}

$EvidencePath = Join-Path $EvidenceDirectory "windows-evidence.json"
$Evidence | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $EvidencePath

if ($env:MTGO_NOTES_MANUAL_EVIDENCE_COMPLETE -ne "1") {
  throw "Automated package checks passed, but required manual Windows evidence is not attested."
}
