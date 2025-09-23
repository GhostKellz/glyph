use crate::protocol::{RequestId, Implementation, ClientCapabilities};
use crate::Result;
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: RequestId,
    pub client_info: Implementation,
    pub capabilities: ClientCapabilities,
    pub created_at: SystemTime,
    pub last_activity: SystemTime,
}

impl Session {
    pub fn new(
        id: RequestId,
        client_info: Implementation,
        capabilities: ClientCapabilities,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id,
            client_info,
            capabilities,
            created_at: now,
            last_activity: now,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<String, Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub async fn create_session(
        &mut self,
        id: RequestId,
        client_info: Implementation,
        capabilities: ClientCapabilities,
    ) -> Result<()> {
        let key = format!("{:?}", id);
        let session = Session::new(id, client_info, capabilities);
        self.sessions.insert(key, session);
        Ok(())
    }

    pub fn get_session(&self, id: &RequestId) -> Option<&Session> {
        let key = format!("{:?}", id);
        self.sessions.get(&key)
    }

    pub fn get_session_mut(&mut self, id: &RequestId) -> Option<&mut Session> {
        let key = format!("{:?}", id);
        self.sessions.get_mut(&key)
    }

    pub fn remove_session(&mut self, id: &RequestId) -> Option<Session> {
        let key = format!("{:?}", id);
        self.sessions.remove(&key)
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn cleanup_inactive_sessions(&mut self, max_idle: std::time::Duration) {
        let now = SystemTime::now();
        self.sessions.retain(|_, session| {
            now.duration_since(session.last_activity)
                .map(|duration| duration < max_idle)
                .unwrap_or(false)
        });
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}