use nocodo_praxis::auth::{PermissionId, RoleId, RoleSemantics};
use nocodo_praxis::provenance::Provenance;
use nocodo_praxis::primitives::AtLeastOne;
use app_backend::praxis::has_permission;

pub use nocodo_praxis::auth::Role;

// ── Provenance ─────────────────────────────────────────────────────────────────

const PRD_CONVERSATION: Provenance = Provenance::Conversation {
    id: "prd-init",
    excerpt: "A simple Todo app. Email/password auth. Anyone who registers can \
               self-assign and self-manage. Admin can create tasks, kick members \
               out, assign to anyone.",
};

fn prov() -> AtLeastOne<Provenance> {
    AtLeastOne { head: PRD_CONVERSATION, tail: &[] }
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
            provenance: prov(),
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

// ── Task State Machine ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Todo,
    InProgress,
    Done,
    Cancelled,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, TaskState::Done | TaskState::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskTransition {
    Start,
    Complete,
    Cancel,
    Reopen,
}

impl TaskTransition {
    pub fn required_permission(self) -> PermissionId {
        match self {
            TaskTransition::Start | TaskTransition::Complete => PERM_UPDATE_OWN_TASK_STATUS,
            TaskTransition::Cancel => PERM_ASSIGN_TASK_TO_ANYONE,
            TaskTransition::Reopen => PERM_ASSIGN_TASK_TO_ANYONE,
        }
    }
}

const TRANSITIONS: &[(TaskState, TaskTransition, TaskState)] = &[
    (TaskState::Todo, TaskTransition::Start, TaskState::InProgress),
    (TaskState::InProgress, TaskTransition::Complete, TaskState::Done),
    (TaskState::Todo, TaskTransition::Cancel, TaskState::Cancelled),
    (TaskState::InProgress, TaskTransition::Cancel, TaskState::Cancelled),
];

pub fn can_transition(
    from: TaskState,
    transition: TaskTransition,
    user_roles: &[RoleId],
    all_roles: &[&Role],
) -> bool {
    if from.is_terminal() {
        return false;
    }

    let valid = TRANSITIONS.iter().any(|&(f, t, _)| f == from && t == transition);
    if !valid {
        return false;
    }

    let required = transition.required_permission();
    has_permission(user_roles, all_roles, &required)
}

pub fn apply_transition(from: TaskState, transition: TaskTransition) -> Option<TaskState> {
    TRANSITIONS
        .iter()
        .find(|&&(f, t, _)| f == from && t == transition)
        .map(|&(_, _, to)| to)
}

pub fn valid_transitions(from: TaskState) -> Vec<TaskTransition> {
    if from.is_terminal() {
        return vec![];
    }
    TRANSITIONS
        .iter()
        .filter(|&&(f, _, _)| f == from)
        .map(|&(_, t, _)| t)
        .collect()
}
