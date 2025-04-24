#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum NodeRole {
    Coordinator,
    Participant,
}

impl Default for NodeRole {
    fn default() -> Self {
         NodeRole::Participant
    }
}
