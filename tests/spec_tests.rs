use todo_app_spec::*;
use app_backend::praxis::has_permission;
use nocodo_praxis::auth::RoleId;
use nocodo_praxis::statemachine::{StateId, TransitionCondition, Transitions};

fn role_refs(roles: &[Role]) -> Vec<&Role> {
    roles.iter().collect()
}

// ── Role Resolution Tests ──────────────────────────────────────────────────────

#[test]
fn admin_inherits_all_member_permissions() {
    let all = all_roles();
    let refs = role_refs(&all);
    let perms = app_backend::praxis::resolve_permissions(
        all.iter().find(|r| r.id == ROLE_ADMIN).unwrap(),
        &refs,
    );
    assert!(perms.contains(&PERM_VIEW_ALL_TASKS));
    assert!(perms.contains(&PERM_SELF_ASSIGN_TASK));
    assert!(perms.contains(&PERM_UPDATE_OWN_TASK_STATUS));
    assert!(perms.contains(&PERM_CREATE_TASK));
    assert!(perms.contains(&PERM_ASSIGN_TASK_TO_ANYONE));
    assert!(perms.contains(&PERM_REMOVE_MEMBER));
}

#[test]
fn member_has_base_permissions_but_not_admin_perms() {
    let all = all_roles();
    let refs = role_refs(&all);
    let perms = app_backend::praxis::resolve_permissions(
        all.iter().find(|r| r.id == ROLE_MEMBER).unwrap(),
        &refs,
    );
    assert!(perms.contains(&PERM_VIEW_ALL_TASKS));
    assert!(perms.contains(&PERM_SELF_ASSIGN_TASK));
    assert!(perms.contains(&PERM_UPDATE_OWN_TASK_STATUS));
    assert!(!perms.contains(&PERM_CREATE_TASK));
    assert!(!perms.contains(&PERM_ASSIGN_TASK_TO_ANYONE));
    assert!(!perms.contains(&PERM_REMOVE_MEMBER));
}

// ── Permission Check Tests ─────────────────────────────────────────────────────

#[test]
fn admin_can_create_task() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(has_permission(&[ROLE_ADMIN], &refs, &PERM_CREATE_TASK));
}

#[test]
fn member_cannot_create_task() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!has_permission(&[ROLE_MEMBER], &refs, &PERM_CREATE_TASK));
}

#[test]
fn member_can_view_all_tasks() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(has_permission(&[ROLE_MEMBER], &refs, &PERM_VIEW_ALL_TASKS));
}

#[test]
fn member_can_self_assign() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(has_permission(&[ROLE_MEMBER], &refs, &PERM_SELF_ASSIGN_TASK));
}

// ── State Machine Tests ────────────────────────────────────────────────────────

#[test]
fn todo_can_transition_to_in_progress() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(S_TODO, S_IN_PROGRESS, &[ROLE_MEMBER], &refs));
}

#[test]
fn todo_cannot_transition_to_done_directly() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!can_transition(S_TODO, S_DONE, &[ROLE_MEMBER], &refs));
}

#[test]
fn done_is_terminal() {
    let states = task_states();
    let done = nocodo_praxis::statemachine::find_state(&S_DONE, &states).unwrap();
    assert!(done.is_terminal());
}

#[test]
fn cancelled_is_terminal() {
    let states = task_states();
    let cancelled = nocodo_praxis::statemachine::find_state(&S_CANCELLED, &states).unwrap();
    assert!(cancelled.is_terminal());
}

#[test]
fn done_cannot_transition() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!can_transition(S_DONE, S_TODO, &[ROLE_ADMIN], &refs));
}

#[test]
fn member_cannot_cancel_tasks() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!can_transition(S_IN_PROGRESS, S_CANCELLED, &[ROLE_MEMBER], &refs));
}

#[test]
fn admin_can_cancel_tasks() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(S_IN_PROGRESS, S_CANCELLED, &[ROLE_ADMIN], &refs));
}

#[test]
fn valid_targets_from_todo() {
    let targets = valid_target_states(S_TODO);
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&S_IN_PROGRESS));
    assert!(targets.contains(&S_CANCELLED));
}

#[test]
fn valid_targets_from_terminal_are_empty() {
    assert!(valid_target_states(S_DONE).is_empty());
    assert!(valid_target_states(S_CANCELLED).is_empty());
}

// ── Provenance Tests ───────────────────────────────────────────────────────────

#[test]
fn view_all_tasks_has_inferred_provenance() {
    let all = all_roles();
    let auth_role = all.iter().find(|r| r.id == ROLE_ALL_AUTHENTICATED).unwrap();
    let first_prov = &auth_role.provenance.head;
    assert!(
        matches!(first_prov, nocodo_praxis::provenance::Provenance::Inferred { .. }),
        "view_all_tasks should have Inferred provenance, not stated in PRD"
    );
}

// ── Entity Tests ───────────────────────────────────────────────────────────────

#[test]
fn task_entity_has_three_fields() {
    let entity = task_entity();
    assert_eq!(entity.fields.len(), 3);
}

#[test]
fn task_entity_title_has_pending_invariant() {
    let entity = task_entity();
    assert!(entity.has_pending_invariants());
}

#[test]
fn task_entity_has_no_states_yet() {
    let entity = task_entity();
    assert!(entity.states.is_empty());
}

#[test]
fn task_states_all_have_terminal() {
    let states = task_states();
    assert!(nocodo_praxis::statemachine::has_terminal_state(&states));
}

// ── Unresolved State Machine Tests ─────────────────────────────────────────────

#[test]
fn unresolved_transitions_exist_and_have_unresolved_condition() {
    let unresolved = unresolved_transitions();
    assert!(!unresolved.is_empty(), "PRD has open questions that should be tracked");
    for transition in &unresolved {
        assert!(
            matches!(transition.condition, nocodo_praxis::statemachine::TransitionCondition::Unresolved(_)),
            "unresolved transitions should use TransitionCondition::Unresolved"
        );
    }
}

#[test]
fn unresolved_transitions_are_not_in_resolved_states() {
    for transition in unresolved_transitions() {
        assert!(
            !can_transition(S_IN_PROGRESS, transition.to, &[ROLE_MEMBER], &[]),
            "unresolved transitions should not pass can_transition"
        );
    }
}

#[test]
#[ignore = "UNRESOLVED: PRD does not specify whether in_progress tasks can revert to todo"]
fn in_progress_can_revert_to_todo() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(S_IN_PROGRESS, S_TODO, &[ROLE_MEMBER], &refs));
}

#[test]
#[ignore = "UNRESOLVED: PRD does not specify whether cancelled tasks can be reopened"]
fn cancelled_can_be_reopened() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(S_CANCELLED, S_TODO, &[ROLE_ADMIN], &refs));
}

// ── Integration-Style: Spec + Runtime Checks ──────────────────────────────────
//
// These tests demonstrate how a controller would compose:
//   1. Spec check (can_transition — roles, permissions, transition validity)
//   2. Runtime check (OnlyIfAssignedToSelf — is the user the task's assignee?)
//
// The spec encodes the rule; the controller enriches it with runtime data.

struct Task {
    assignee_id: u64,
}

fn can_user_transition_task(
    task: &Task,
    user_id: u64,
    from: StateId,
    to: StateId,
    user_roles: &[RoleId],
    all_roles: &[&Role],
) -> bool {
    // Step 1 — structural check: does the spec allow this transition for this role?
    if !can_transition(from, to, user_roles, all_roles) {
        return false;
    }

    // Step 2 — runtime check: OnlyIfAssignedToSelf means the user must be the assignee.
    // The spec captures this as TransitionCondition, but can't resolve assignee data.
    // The controller reads the task from the DB and checks ownership here.
    let states = task_states();
    let state = match nocodo_praxis::statemachine::find_state(&from, &states) {
        Some(s) => s,
        None => return false,
    };
    if let Transitions::To(list) = &state.transitions {
        for t in list.all() {
            if t.to == to {
                if matches!(t.condition, TransitionCondition::OnlyIfAssignedToSelf) {
                    return task.assignee_id == user_id;
                }
                return true;
            }
        }
    }
    false
}

#[test]
fn member_can_start_own_task() {
    let all = all_roles();
    let refs = role_refs(&all);
    let task = Task { assignee_id: 42 };
    assert!(can_user_transition_task(
        &task, 42, S_TODO, S_IN_PROGRESS, &[ROLE_MEMBER], &refs,
    ));
}

#[test]
fn member_cannot_start_someone_elses_task() {
    let all = all_roles();
    let refs = role_refs(&all);
    let task = Task { assignee_id: 99 };
    assert!(!can_user_transition_task(
        &task, 42, S_TODO, S_IN_PROGRESS, &[ROLE_MEMBER], &refs,
    ));
}

#[test]
fn member_cannot_complete_someone_elses_task() {
    let all = all_roles();
    let refs = role_refs(&all);
    let task = Task { assignee_id: 99 };
    assert!(!can_user_transition_task(
        &task, 42, S_IN_PROGRESS, S_DONE, &[ROLE_MEMBER], &refs,
    ));
}

#[test]
fn admin_can_cancel_any_task_regardless_of_assignee() {
    let all = all_roles();
    let refs = role_refs(&all);
    let task = Task { assignee_id: 99 };
    assert!(can_user_transition_task(
        &task, 42, S_TODO, S_CANCELLED, &[ROLE_ADMIN], &refs,
    ));
}

#[test]
fn assignee_check_bypassed_when_condition_is_always() {
    let all = all_roles();
    let refs = role_refs(&all);
    let task = Task { assignee_id: 99 };
    assert!(can_user_transition_task(
        &task, 42, S_IN_PROGRESS, S_CANCELLED, &[ROLE_ADMIN], &refs,
    ));
}
