# Windows release validation

Task 07 release evidence is intentionally split into automated and manual gates.

`validate-windows.ps1` runs only on labeled self-hosted Windows 10 22H2 and
Windows 11 x64 runners after the complete repository gate and NSIS build. It
records the OS, architecture, WebView2 presence, artifact name, Authenticode
status, and the automated contract families exercised by the run.

The script fails closed unless `MTGO_NOTES_MANUAL_EVIDENCE_COMPLETE=1` is set by
the controlled release environment. Production signing also requires
`MTGO_NOTES_REQUIRE_PRODUCTION_SIGNATURE=1`; an unsigned or invalid artifact
then fails immediately.

The manual attestation must cover:

- real selected MTGO UIA and cropped OCR profiles;
- DPI 100%, 125%, 150%, and 200% plus multi-monitor placement;
- non-activating overlay, tray, global shortcut, and quick capture;
- Windows Narrator/keyboard operation and visible focus;
- SQLCipher/DPAPI, offline restart, backup/restore/export, and no-network
  diagnostics;
- signed updater success, tamper rejection, interruption, and last-known-good
  recovery;
- capture, restricted disclosure, search, classification, detector CPU, memory,
  and startup performance budgets.

Evidence must contain no real opponent handle, note, OCR text, screenshot,
source URL, secret, passphrase, notebook path, or decklist.
