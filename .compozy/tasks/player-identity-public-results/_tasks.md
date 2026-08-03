---
schema_version: "compozy.tasks/v2"
workflow: player-identity-public-results
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
  edges:
    - from: task_01
      to: task_02
    - from: task_02
      to: task_03
    - from: task_03
      to: task_04
    - from: task_04
      to: task_05
---

# Player Identity and Public Results Task List

This graph implements the approved Player Identity and Public Results V1 handoff as
five robust vertical slices. An edge means the source task must finish before the
destination task begins.

| Task | Title | Type | Complexity | Assigned tests |
|---|---|---|---|---:|
| `task_01` | Establish the Player Bounded-Context Persistence Foundation | backend | critical | 21 |
| `task_02` | Build the Fail-Closed Public Source Runtime | backend | critical | 50 |
| `task_03` | Deliver the Immutable Evidence Lifecycle and Classification | backend | critical | 29 |
| `task_04` | Extend Player Deletion, Portability, and Export | backend | critical | 23 |
| `task_05` | Integrate the Accessible Player Workspace | frontend | critical | 39 |

Local completion, packaged Windows evidence, and live Census enablement remain
separate gates. In particular, `E2E-015` and `E2E-016` cannot be marked passing from
macOS or synthetic-provider evidence.
