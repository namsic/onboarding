use crate::config::*;

struct ClusterMessage {
    term: TermSize,
    node_id: ClusterSize,
    message_type: MessageType,
}

pub enum MessageType {
    Vote(Option<u8>),
    Heartbeat(u8),
}
