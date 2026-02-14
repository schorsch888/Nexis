//! Permission domain extensions for Nexis.

pub use nexis_protocol::{Action, Permissions};

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct PermissionChecker {
    permissions: Permissions,
}

impl PermissionChecker {
    pub fn new(permissions: Permissions) -> Self {
        Self { permissions }
    }

    pub fn can_read(&self) -> bool {
        self.permissions.can(Action::Read)
    }

    pub fn can_write(&self) -> bool {
        self.permissions.can(Action::Write)
    }

    pub fn can_invoke(&self) -> bool {
        self.permissions.can(Action::Invoke)
    }

    pub fn is_admin(&self) -> bool {
        self.permissions.can(Action::Admin)
    }

    pub fn can_access_room(&self, room_id: &str) -> bool {
        self.permissions.can_access_room(room_id)
    }

    pub fn effective_permissions(&self, room_id: &str) -> HashSet<Action> {
        let mut actions = HashSet::new();
        if !self.can_access_room(room_id) {
            return actions;
        }
        for action in [Action::Read, Action::Write, Action::Invoke, Action::Admin] {
            if self.permissions.can(action) {
                actions.insert(action);
            }
        }
        actions
    }
}

impl From<Permissions> for PermissionChecker {
    fn from(permissions: Permissions) -> Self {
        Self::new(permissions)
    }
}
