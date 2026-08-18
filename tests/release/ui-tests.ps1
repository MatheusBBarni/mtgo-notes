# Requires a running unpackaged MTGONotes.App process on Windows.
# Usage: .\ui-tests.ps1 -AppPid <PID>
param([Parameter(Mandatory)][int]$AppPid)

$ErrorActionPreference = 'Continue'
$pass = 0
$fail = 0
$results = @()
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$shots = Join-Path $here 'screenshots'

$windows = winapp ui list-windows -a $AppPid --json 2>$null | ConvertFrom-Json
$hwnd = ($windows | Where-Object { $_.title -ne 'PopupHost' } | Select-Object -First 1).hwnd

function Test-UI {
    param([string]$Name, [scriptblock]$Script)
    try {
        $output = & $Script 2>&1
        if ($LASTEXITCODE -eq 0) {
            $script:pass++
            $script:results += @{ name = $Name; status = 'PASS' }
        } else {
            $script:fail++
            $script:results += @{ name = $Name; status = 'FAIL'; detail = "$output" }
        }
    } catch {
        $script:fail++
        $script:results += @{ name = $Name; status = 'FAIL'; detail = "$_" }
    }
}

Test-UI 'NavEncounter exists' { winapp ui wait-for 'NavEncounter' -a $AppPid -t 4000 }
Test-UI 'NavHistory exists' { winapp ui wait-for 'NavHistory' -a $AppPid -t 3000 }
Test-UI 'NavSettings exists' { winapp ui wait-for 'NavSettings' -a $AppPid -t 3000 }
Test-UI 'Encounter handle exists' { winapp ui wait-for 'TxtOpponentHandle' -a $AppPid -t 3000 }
Test-UI 'Confirm opponent exists' { winapp ui wait-for 'BtnConfirmOpponent' -a $AppPid -t 3000 }
Test-UI 'Notes empty state' { winapp ui wait-for 'LblNotesEmpty' -a $AppPid -t 3000 }

Test-UI 'Navigate to History' { winapp ui invoke 'NavHistory' -a $AppPid }
Test-UI 'History search exists' { winapp ui wait-for 'TxtHistoryQuery' -a $AppPid -t 3000 }
Test-UI 'History list exists' { winapp ui wait-for 'LstHistory' -a $AppPid -t 3000 }
Test-UI 'Export exists' { winapp ui wait-for 'BtnExportText' -a $AppPid -t 3000 }
Test-UI 'Backup exists' { winapp ui wait-for 'BtnBackup' -a $AppPid -t 3000 }

Test-UI 'Navigate to Settings' { winapp ui invoke 'NavSettings' -a $AppPid }
Test-UI 'Live attach toggle exists' { winapp ui wait-for 'TglLiveAttach' -a $AppPid -t 3000 }
Test-UI 'Overlay toggle exists' { winapp ui wait-for 'TglOverlay' -a $AppPid -t 3000 }
Test-UI 'Theme selector exists' { winapp ui wait-for 'RbnTheme' -a $AppPid -t 3000 }

Test-UI 'Return to Encounter' { winapp ui invoke 'NavEncounter' -a $AppPid }
Test-UI 'Encounter restored' { winapp ui wait-for 'TxtOpponentHandle' -a $AppPid -t 3000 }

$allElements = (winapp ui inspect -a $AppPid --interactive --json 2>$null | ConvertFrom-Json).elements
$appElements = @($allElements | Where-Object {
    $_.type -match 'Button|TextBox|ComboBox|CheckBox|ToggleSwitch|TabItem|Edit|RadioButton' -and
    $_.name -notmatch 'Minimize|Maximize|Close|System' -and
    $_.className -notmatch 'PickerHost|#32770|CabinetWClass'
})
$missingId = @($appElements | Where-Object { -not $_.automationId })
if ($missingId.Count -eq 0) {
    $pass++
    $results += @{ name = 'All app controls have AutomationId'; status = 'PASS' }
} else {
    $fail++
    $names = ($missingId | ForEach-Object { "$($_.type) '$($_.name)'" }) -join ', '
    $results += @{ name = 'AutomationId coverage'; status = 'FAIL'; detail = "Missing: $names" }
}

New-Item -ItemType Directory -Force -Path $shots | Out-Null
winapp ui screenshot -a $AppPid -o (Join-Path $shots '01-encounter.png') 2>$null
winapp ui invoke 'NavHistory' -a $AppPid | Out-Null
Start-Sleep -Milliseconds 400
winapp ui screenshot -a $AppPid -o (Join-Path $shots '02-history.png') 2>$null
winapp ui invoke 'NavSettings' -a $AppPid | Out-Null
Start-Sleep -Milliseconds 400
winapp ui screenshot -a $AppPid -o (Join-Path $shots '03-settings.png') 2>$null

Write-Host "`nPassed: $pass | Failed: $fail"
$results | Where-Object { $_.status -eq 'FAIL' } | ForEach-Object {
    Write-Host "  FAIL: $($_.name) — $($_.detail)"
}
$results | ConvertTo-Json | Out-File (Join-Path $here 'ui-test-results.json')
if ($fail -gt 0) { exit 1 } else { exit 0 }
