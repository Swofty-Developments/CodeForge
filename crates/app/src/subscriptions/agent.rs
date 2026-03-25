use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use codeforge_session::{AgentEvent, SessionId};
use iced::Subscription;
use tokio::sync::{mpsc, Mutex};

use crate::message::{AgentMessage, Message};

/// Shared state holding event receivers for active sessions.
#[derive(Debug, Clone, Default)]
pub struct AgentEventReceivers {
    inner: Arc<Mutex<HashMap<SessionId, mpsc::UnboundedReceiver<AgentEvent>>>>,
}

// We use a constant hash since there is only ever one subscription instance.
impl Hash for AgentEventReceivers {
    fn hash<H: Hasher>(&self, state: &mut H) {
        "agent-event-receivers".hash(state);
    }
}

impl AgentEventReceivers {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn insert(&self, id: SessionId, rx: mpsc::UnboundedReceiver<AgentEvent>) {
        self.inner.lock().await.insert(id, rx);
    }

    pub async fn remove(&self, id: &SessionId) {
        self.inner.lock().await.remove(id);
    }
}

/// Create an iced Subscription that polls all active session event receivers.
pub fn agent_events(receivers: AgentEventReceivers) -> Subscription<Message> {
    Subscription::run_with(receivers, create_agent_stream)
}

fn create_agent_stream(
    receivers: &AgentEventReceivers,
) -> impl futures::Stream<Item = Message> {
    let receivers = receivers.clone();
    futures::stream::unfold(receivers, |receivers| async move {
        loop {
            let mut inner = receivers.inner.lock().await;

            if inner.is_empty() {
                drop(inner);
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                continue;
            }

            let session_ids: Vec<SessionId> = inner.keys().copied().collect();
            let mut closed_sessions = Vec::new();

            for session_id in &session_ids {
                if let Some(rx) = inner.get_mut(session_id) {
                    match rx.try_recv() {
                        Ok(event) => {
                            let msg = Message::Agent(AgentMessage::Event {
                                session_id: *session_id,
                                event,
                            });
                            drop(inner);
                            return Some((msg, receivers));
                        }
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            closed_sessions.push(*session_id);
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {}
                    }
                }
            }

            for id in closed_sessions {
                inner.remove(&id);
            }

            drop(inner);
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        }
    })
}
