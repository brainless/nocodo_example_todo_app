use nocodo_praxis::auth::{PermissionId, RoleId, RoleSemantics};
use nocodo_praxis::entity::{Entity, EntityId, Field};
use nocodo_praxis::primitives::{AtLeastOne, Unresolved};
use nocodo_praxis::provenance::Provenance;
use nocodo_praxis::statemachine::{
    State, StateId, Transition, TransitionCondition, Transitions,
};
use app_backend::praxis::has_permission;

pub use nocodo_praxis::auth::Role;

// ── Provenance ─────────────────────────────────────────────────────────────────

const PRD_CONVERSATION: Provenance = Provenance::Conversation {
    id: "prd-init",
    excerpt: "A simple Todo app. Email/password auth. Anyone who registers can \
               self-assign and self-manage. Admin can create tasks, kick members \
               out, assign to anyone.",
};

const INFERRED_VIEW_ALL: Provenance = Provenance::Inferred {
    reason: "Team coordination requires shared visibility",
    from: &["prd-init"],
};

fn prov() -> AtLeastOne<Provenance> {
    AtLeastOne { head: PRD_CONVERSATION, tail: &[] }
}

fn prov_inferred() -> AtLeastOne<Provenance> {
    AtLeastOne { head: INFERRED_VIEW_ALL, tail: &[] }
}

// ── Permissions ────────────────────────────────────────────────────────────────

pub const PERM_VIEW_ALL_TASKS: PermissionId = PermissionId("view_all_tasks");
pub const PERM_SELF_ASSIGN_TASK: PermissionId = PermissionId("self_assign_task");
pub const PERM_UPDATE_OWN_TASK_STATUS: PermissionId = PermissionId("update_own_task_status");
pub const PERM_CREATE_TASK: PermissionId = PermissionId("create_task");
pub const PERM_ASSIGN_TASK_TO_ANYONE: PermissionId = PermissionId("assign_task_to_anyone");
pub const PERM_REMOVE_MEMBER: PermissionId = PermissionId("remove_member");

// ── Roles ──────────────────────────────────────────────────────────────────────

pub const ROLE_ALL_AUTHENTICATED: RoleId = RoleId("all_authenticated");
pub const ROLE_MEMBER: RoleId = RoleId("member");
pub const ROLE_ADMIN: RoleId = RoleId("admin");

pub fn all_roles() -> [Role; 3] {
    [
        Role {
            id: ROLE_ALL_AUTHENTICATED,
            description: "Any user with a valid session",
            semantics: RoleSemantics::Flat,
            permissions: &[PERM_VIEW_ALL_TASKS],
            personas: &[],
            provenance: prov_inferred(),
        },
        Role {
            id: ROLE_MEMBER,
            description: "Registered team member",
            semantics: RoleSemantics::Inherits { parent: ROLE_ALL_AUTHENTICATED },
            permissions: &[PERM_SELF_ASSIGN_TASK, PERM_UPDATE_OWN_TASK_STATUS],
            personas: &[],
            provenance: prov(),
        },
        Role {
            id: ROLE_ADMIN,
            description: "Team admin",
            semantics: RoleSemantics::Inherits { parent: ROLE_MEMBER },
            permissions: &[PERM_CREATE_TASK, PERM_ASSIGN_TASK_TO_ANYONE, PERM_REMOVE_MEMBER],
            personas: &[],
            provenance: prov(),
        },
    ]
}

// ── State IDs ──────────────────────────────────────────────────────────────────

pub const S_TODO: StateId = StateId("todo");
pub const S_IN_PROGRESS: StateId = StateId("in_progress");
pub const S_DONE: StateId = StateId("done");
pub const S_CANCELLED: StateId = StateId("cancelled");

pub fn task_states() -> [State; 4] {
    [
        State {
            id: S_TODO,
            description: "Task created, not yet started",
            transitions: Transitions::To(AtLeastOne {
                head: Transition {
                    to: S_IN_PROGRESS,
                    permitted_roles: AtLeastOne { head: ROLE_MEMBER, tail: &[ROLE_ADMIN] },
                    condition: TransitionCondition::OnlyIfAssignedToSelf,
                    provenance: &[PRD_CONVERSATION],
                },
                tail: &[Transition {
                    to: S_CANCELLED,
                    permitted_roles: AtLeastOne { head: ROLE_ADMIN, tail: &[] },
                    condition: TransitionCondition::Always,
                    provenance: &[PRD_CONVERSATION],
                }],
            }),
            provenance: &[PRD_CONVERSATION],
        },
        State {
            id: S_IN_PROGRESS,
            description: "Task actively being worked on",
            transitions: Transitions::To(AtLeastOne {
                head: Transition {
                    to: S_DONE,
                    permitted_roles: AtLeastOne { head: ROLE_MEMBER, tail: &[ROLE_ADMIN] },
                    condition: TransitionCondition::OnlyIfAssignedToSelf,
                    provenance: &[PRD_CONVERSATION],
                },
                tail: &[Transition {
                    to: S_CANCELLED,
                    permitted_roles: AtLeastOne { head: ROLE_ADMIN, tail: &[] },
                    condition: TransitionCondition::Always,
                    provenance: &[PRD_CONVERSATION],
                }],
            }),
            provenance: &[PRD_CONVERSATION],
        },
        State {
            id: S_DONE,
            description: "Task completed. Terminal.",
            transitions: Transitions::Terminal,
            provenance: &[PRD_CONVERSATION],
        },
        State {
            id: S_CANCELLED,
            description: "Task cancelled. Terminal.",
            transitions: Transitions::Terminal,
            provenance: &[PRD_CONVERSATION],
        },
    ]
}

/// Transitions that need PRD clarification before they can be added.
pub fn unresolved_transitions() -> Vec<Transition> {
    const INFERRED_PROV: Provenance = Provenance::Inferred {
        reason: "Common product pattern; not stated in PRD",
        from: &["prd-init"],
    };
    static PROV_SLICE: &[Provenance] = &[INFERRED_PROV];
    vec![
        Transition {
            to: S_TODO,
            permitted_roles: AtLeastOne { head: ROLE_MEMBER, tail: &[ROLE_ADMIN] },
            condition: TransitionCondition::Unresolved(
                "PRD does not specify whether in_progress tasks can revert to todo",
            ),
            provenance: PROV_SLICE,
        },
        Transition {
            to: S_TODO,
            permitted_roles: AtLeastOne { head: ROLE_ADMIN, tail: &[] },
            condition: TransitionCondition::Unresolved(
                "PRD does not specify whether cancelled tasks can be reopened",
            ),
            provenance: PROV_SLICE,
        },
    ]
}

// ── Task Entity ────────────────────────────────────────────────────────────────

pub fn task_entity() -> Entity {
    Entity {
        id: EntityId("task"),
        description: "A unit of work that can be assigned and tracked",
        fields: &[
            Field {
                name: "title",
                description: "Short human-readable name for the task",
                invariants: &[Unresolved::Pending {
                    reason: "Maximum title length not specified in PRD",
                    provenance: AtLeastOne {
                        head: Provenance::Inferred {
                            reason: "All text fields require a length bound for storage",
                            from: &["prd-init"],
                        },
                        tail: &[],
                    },
                }],
                provenance: &[PRD_CONVERSATION],
            },
            Field {
                name: "status",
                description: "Current lifecycle state of the task",
                invariants: &[],
                provenance: &[PRD_CONVERSATION],
            },
            Field {
                name: "assignee",
                description: "Who is responsible for this task",
                invariants: &[],
                provenance: &[PRD_CONVERSATION],
            },
        ],
        states: &[],
        invariants: &[],
        provenance: &[PRD_CONVERSATION],
    }
}

// ── State Machine Helpers ──────────────────────────────────────────────────────

pub fn can_transition(
    from: StateId,
    to: StateId,
    user_roles: &[RoleId],
    all_roles: &[&Role],
) -> bool {
    let states = task_states();
    let state = match nocodo_praxis::statemachine::find_state(&from, &states) {
        Some(s) => s,
        None => return false,
    };

    let transitions = match &state.transitions {
        Transitions::Terminal => return false,
        Transitions::To(list) => list,
    };

    for transition in transitions.all() {
        if transition.to == to {
            if !transition.permitted_roles.contains(|rid| {
                user_roles.iter().any(|ur| ur == rid)
            }) {
                return false;
            }
            if !check_condition(&transition.condition, user_roles, all_roles) {
                return false;
            }
            return true;
        }
    }
    false
}

fn check_condition(
    condition: &TransitionCondition,
    user_roles: &[RoleId],
    all_roles: &[&Role],
) -> bool {
    match condition {
        TransitionCondition::Always => true,
        TransitionCondition::OnlyIfAssignedToSelf => true,
        TransitionCondition::OnlyIfAssignedTo(rid) => {
            user_roles.iter().any(|ur| ur == rid)
        }
        TransitionCondition::RequiresPermission(perm) => {
            has_permission(user_roles, all_roles, perm)
        }
        TransitionCondition::All(conditions) => {
            conditions.iter().all(|c| check_condition(c, user_roles, all_roles))
        }
        TransitionCondition::Any(conditions) => {
            conditions.iter().any(|c| check_condition(c, user_roles, all_roles))
        }
        TransitionCondition::Unresolved(_) => false,
    }
}

pub fn valid_target_states(from: StateId) -> Vec<StateId> {
    let states = task_states();
    let state = match nocodo_praxis::statemachine::find_state(&from, &states) {
        Some(s) => s,
        None => return vec![],
    };
    match &state.transitions {
        Transitions::Terminal => vec![],
        Transitions::To(list) => list.all().map(|t| t.to).collect(),
    }
}
