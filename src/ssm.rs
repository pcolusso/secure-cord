use tokio::sync::{oneshot, mpsc};

struct SessionAcor {
    receiver: mpsc::Receiver<SessionMessage>,
    task: JoinHandle<()>
}

impl SessionActor {}
