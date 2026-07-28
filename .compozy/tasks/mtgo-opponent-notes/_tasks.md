---
schema_version: "compozy.tasks/v2"
workflow: mtgo-opponent-notes
graph:
  nodes:
    - id: task_01
      file: task_01.md
    - id: task_02
      file: task_02.md
    - id: task_03
      file: task_03.md
    - id: task_04
      file: task_04.md
    - id: task_05
      file: task_05.md
    - id: task_06
      file: task_06.md
    - id: task_07
      file: task_07.md
  edges:
    - from: task_01
      to: task_02
    - from: task_02
      to: task_03
    - from: task_03
      to: task_04
    - from: task_03
      to: task_05
    - from: task_04
      to: task_06
    - from: task_05
      to: task_06
    - from: task_06
      to: task_07
---

# MTGO Opponent Notes Task List

## Execution Waves

1. `task_01` — establish the secure Tauri workspace, window boundaries, IPC contracts, and design foundation.
2. `task_02` — implement the encrypted notebook, encounter domain, disclosure policy, and durable operation primitives.
3. `task_03` — deliver automatic context detection and rapid in-match capture on the trusted core.
4. `task_04`, `task_05` — build notebook workflows and official-deck enrichment/classification in parallel.
5. `task_06` — add encrypted portability and text export after all durable data producers exist.
6. `task_07` — finish diagnostics, opt-in updates, offline resilience, and packaged Windows evidence.

## Task Summary

| ID | Title | Type | Complexity | Depends on | Test IDs |
| --- | --- | --- | --- | --- | ---: |
| task_01 | Scaffold the secure multi-window desktop foundation | infra | high | — | 16 |
| task_02 | Implement the encrypted notebook and policy core | backend | critical | task_01 | 41 |
| task_03 | Deliver automatic match context and rapid in-match capture | backend | critical | task_02 | 132 |
| task_04 | Deliver the personal notebook, history, identity, and deletion workflows | frontend | high | task_03 | 81 |
| task_05 | Deliver official deck enrichment and local archetype classification | backend | high | task_03 | 55 |
| task_06 | Deliver encrypted backup, staged restore, and text export | backend | critical | task_04, task_05 | 58 |
| task_07 | Ship private diagnostics, opt-in updates, offline resilience, and Windows release validation | infra | critical | task_06 | 38 |

