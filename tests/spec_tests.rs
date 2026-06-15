use todo_app_spec::*;
use app_backend::praxis::has_permission;

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
    assert!(can_transition(
        TaskState::Todo,
        TaskTransition::Start,
        &[ROLE_MEMBER],
        &refs,
    ));
}

#[test]
fn todo_cannot_transition_to_done_directly() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!can_transition(
        TaskState::Todo,
        TaskTransition::Complete,
        &[ROLE_MEMBER],
        &refs,
    ));
}

#[test]
fn done_is_terminal() {
    assert!(TaskState::Done.is_terminal());
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!can_transition(
        TaskState::Done,
        TaskTransition::Start,
        &[ROLE_ADMIN],
        &refs,
    ));
}

#[test]
fn cancelled_is_terminal() {
    assert!(TaskState::Cancelled.is_terminal());
}

#[test]
fn member_cannot_cancel_tasks() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(!can_transition(
        TaskState::InProgress,
        TaskTransition::Cancel,
        &[ROLE_MEMBER],
        &refs,
    ));
}

#[test]
fn admin_can_cancel_tasks() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(
        TaskState::InProgress,
        TaskTransition::Cancel,
        &[ROLE_ADMIN],
        &refs,
    ));
}

#[test]
fn apply_transition_works() {
    assert_eq!(
        apply_transition(TaskState::Todo, TaskTransition::Start),
        Some(TaskState::InProgress)
    );
    assert_eq!(
        apply_transition(TaskState::Todo, TaskTransition::Complete),
        None
    );
}

#[test]
fn valid_transitions_from_todo() {
    let transitions = valid_transitions(TaskState::Todo);
    assert_eq!(transitions.len(), 2);
    assert!(transitions.contains(&TaskTransition::Start));
    assert!(transitions.contains(&TaskTransition::Cancel));
}

#[test]
fn valid_transitions_from_terminal_are_empty() {
    assert!(valid_transitions(TaskState::Done).is_empty());
    assert!(valid_transitions(TaskState::Cancelled).is_empty());
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

// ── Unresolved State Machine Tests ─────────────────────────────────────────────

#[test]
fn unresolved_transitions_exist_and_are_pending() {
    let unresolved = unresolved_transitions();
    assert!(!unresolved.is_empty(), "PRD has open questions that should be tracked");
    for (_, _, t) in &unresolved {
        assert!(t.blocks_codegen(), "every unresolved transition should block codegen");
        assert!(t.reason().is_some(), "every unresolved transition should have a reason");
    }
}

#[test]
fn unresolved_transitions_are_not_in_resolved_set() {
    let unresolved = unresolved_transitions();
    for (from, transition, _) in &unresolved {
        assert!(
            apply_transition(*from, *transition).is_none(),
            "unresolved transition {:?} -> {:?} should not be in TRANSITIONS",
            from, transition
        );
    }
}

#[test]
#[ignore = "UNRESOLVED: PRD does not specify whether in_progress tasks can revert to todo"]
fn in_progress_can_revert_to_todo() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(
        TaskState::InProgress,
        TaskTransition::Unstart,
        &[ROLE_MEMBER],
        &refs,
    ));
}

#[test]
#[ignore = "UNRESOLVED: PRD does not specify whether cancelled tasks can be reopened"]
fn cancelled_can_be_reopened() {
    let all = all_roles();
    let refs = role_refs(&all);
    assert!(can_transition(
        TaskState::Cancelled,
        TaskTransition::Reopen,
        &[ROLE_ADMIN],
        &refs,
    ));
}
