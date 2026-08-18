# Live attach checklist (Windows + MTGO)

This is manual evidence. macOS test passes are not a substitute.

1. Start MTGO and log in. Do **not** let this app type credentials.
2. Launch `MTGONotes.App`. Overlay should appear without stealing focus, show the brand icon, and be draggable from its header. Minimize collapses it to the header; Hide removes it until the tray toggle.
3. Queue or join a match. Overlay should show `Detected opponent: <handle>`.
4. Confirm the opponent. Notebook should create/reuse the profile.
5. When game 1 starts, phase should become in-game restricted. History search stays blocked.
6. Between games, phase should become sideboarding / between games.
7. Kill MTGO. Live status should drop; the active encounter must not be deleted.
8. Relaunch MTGO and log in again. A new provider session must start; an obsolete candidate cannot confirm.
9. Pause live attach from the session/settings path. Overlay fails closed (no new historical data).
10. Confirm there is no login prompt and no chat/queue/concede action from this app.
