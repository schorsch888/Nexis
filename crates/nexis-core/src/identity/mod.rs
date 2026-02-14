//! Identity domain extensions for Nexis.

pub use nexis_protocol::MemberId;

#[derive(Debug, Clone)]
pub struct Identity {
    pub id: MemberId,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl Identity {
    pub fn new(id: MemberId) -> Self {
        Self {
            id,
            display_name: None,
            avatar_url: None,
        }
    }

    pub fn with_display_name(mut self, name: String) -> Self {
        self.display_name = Some(name);
        self
    }

    pub fn with_avatar(mut self, url: String) -> Self {
        self.avatar_url = Some(url);
        self
    }
}
