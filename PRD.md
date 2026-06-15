# Todo App — PRD

**Status**: Reference only. Do not edit. All iteration happens in code.
**Source**: agents/RUNTIME.md §4 (Todo App Worked Example)

## Summary

A simple Todo app. Email/password auth. Anyone who registers can self-assign and self-manage. Admin can create tasks, kick members out, assign to anyone.

## Personas

- **Member** — Anyone who has registered and confirmed their account
- **Admin** — Elevated user who manages team and task assignment

## Permissions

| Permission | Who has it | Notes |
|---|---|---|
| `view_all_tasks` | All authenticated users | Team coordination implies shared visibility |
| `self_assign_task` | Member, Admin | Assign an unassigned task to oneself |
| `update_own_task_status` | Member, Admin | Transition status of a task assigned to oneself |
| `create_task` | Admin only | Create a new task |
| `assign_task_to_anyone` | Admin only | Assign any task to any member |
| `remove_member` | Admin only | Remove a member from the team |

## Roles

- **all_authenticated** (Flat): `view_all_tasks`
- **member** (Inherits all_authenticated): `self_assign_task`, `update_own_task_status`
- **admin** (Inherits member): `create_task`, `assign_task_to_anyone`, `remove_member`

## Task State Machine

```
todo ──→ in_progress ──→ done
  │          │
  └──→ cancelled ←──────┘
```

### Transitions

| From | To | Who | Condition |
|---|---|---|---|
| todo | in_progress | member, admin | Only if assigned to self |
| todo | cancelled | admin | Always |
| in_progress | done | member, admin | Only if assigned to self |
| in_progress | cancelled | admin | Always |
| in_progress | todo | ? | Unresolved — can tasks be un-started? |
| done | — | — | Terminal |
| cancelled | — | — | Terminal — can cancelled be reopened? Unresolved |

## Entities

### Task
- `title` (text, length TBD — Unresolved)
- `status` (state: todo | in_progress | done | cancelled)
- `assignee` (user ID or unassigned)
