use futures::future::Either;
use std::process::ExitStatus;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot};

enum SessionMessage {
    Start,
    Stop,
    Healthy(oneshot::Sender<bool>),
    Stdout(oneshot::Sender<Vec<String>>),
    Stderr(oneshot::Sender<Vec<String>>),
}

enum SessionStatus {
    Fresh,
    Running(Child, BufReader<ChildStdout>, BufReader<ChildStderr>),
    Stopped(Result<ExitStatus, std::io::Error>),
}

struct SessionActor {
    reciever: mpsc::Receiver<SessionMessage>,
    status: SessionStatus,
    target: String,
    host_port: usize,
    dest_port: usize,
    stdout: Vec<String>,
    stderr: Vec<String>,
    env: String
}

impl SessionActor {
    fn new(
        reciever: mpsc::Receiver<SessionMessage>,
        target: String,
        env: String,
        host_port: usize,
        dest_port: usize,
    ) -> Self {
        Self {
            reciever,
            status: SessionStatus::Fresh,
            target,
            host_port,
            dest_port,
            env,
            stdout: vec![],
            stderr: vec![],
        }
    }

    fn handle_message(&mut self, msg: SessionMessage) {
        match msg {
            SessionMessage::Stop => {
                match std::mem::replace(&mut self.status, SessionStatus::Fresh) {
                    SessionStatus::Running(mut child, _, _) => {
                        tokio::spawn(async move {
                            if let Err(e) = child.kill().await {
                                // TODO: HAndle?
                                //eprintln!("Failed to kill child process: {}", e);
                            }
                        });

                        // TODO: Probably use the proper stopped state.
                        self.status = SessionStatus::Fresh;
                    }
                    other_state => self.status = other_state,
                }
            }
            SessionMessage::Start => {
                let mut command = Command::new("aws");
                command.args([
                    "ssm",
                    "start-session",
                    "--target",
                    &self.target,
                    "--document-name",
                    "AWS-StartPortForwardingSession",
                    "--parameters",
                    &format!(
                        "portNumber={},localPortNumber={}",
                        self.dest_port, self.host_port
                    ),
                ]);
                command.env("AWS_PROFILE", &self.env);
                command.stdout(std::process::Stdio::piped());
                command.stderr(std::process::Stdio::piped());
                let res = command.spawn();
                match res {
                    Ok(mut child) => {
                        let stdout = BufReader::new(child.stdout.take().unwrap());
                        let stderr = BufReader::new(child.stderr.take().unwrap());
                        self.status = SessionStatus::Running(child, stdout, stderr);
                    }
                    Err(err) => {
                        self.status = SessionStatus::Stopped(Err(err));
                    }
                }
            }
            SessionMessage::Healthy(reply) => {
                let res = match &self.status {
                    SessionStatus::Fresh | SessionStatus::Stopped(_) => false,
                    SessionStatus::Running(child, _, _) => true,
                };
                reply.send(res).unwrap();
            }
            SessionMessage::Stdout(reply) => {
                reply.send(self.stdout.clone()).unwrap();
            }
            SessionMessage::Stderr(reply) => {
                reply.send(self.stderr.clone()).unwrap();
            }
        }
    }
}

async fn run(mut actor: SessionActor) {
    loop {
        // So, I've really created a mess here. I thought I was being smart by making unrepresentable states impossible,
        // using an enum, but now this has become a right proper clusterfuck. This is necessary to lift out the futures
        // stuck inside the enum, we basically make one that instantly closes if it's not ready.
        let (child_fut, mut stdout_lines, mut stderr_lines) = match &mut actor.status {
            SessionStatus::Running (child, stdout, stderr ) => (
                Either::Left(async move { child.wait().await }),
                Some(stdout.lines()),
                Some(stderr.lines()),
            ),
            _ => (Either::Right(futures::future::pending()), None, None),
        };

        tokio::select! {
            Some(msg) = actor.reciever.recv() => {
                actor.handle_message(msg);
            }

            status = child_fut => {
                actor.status = SessionStatus::Stopped(status)
            }

            line = async {
                if let Some(ref mut lines) = stdout_lines {
                    lines.next_line().await
                } else {
                    std::future::pending().await
                }
            } => {
                if let Ok(Some(line)) = line {
                    actor.stdout.push(line);
                }
            }

            line = async {
                if let Some(ref mut lines) = stderr_lines {
                    lines.next_line().await
                } else {
                    std::future::pending().await
                }
            } => {
                if let Ok(Some(line)) = line {
                    actor.stderr.push(line);
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct Session {
    sender: mpsc::Sender<SessionMessage>,
}

impl Session {
    pub fn new(target: String, env: String, host_port: usize, dest_port: usize) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = SessionActor::new(receiver, target, env, host_port, dest_port);
        tokio::spawn(run(actor));

        Self { sender }
    }

    pub async fn start(&self) {
        self.sender
            .send(SessionMessage::Start)
            .await
            .expect("Actor dead?");
    }

    pub async fn stop(&self) {
        self.sender
            .send(SessionMessage::Stop)
            .await
            .expect("Actor dead?");
    }

    pub async fn healthy(&self) -> bool {
        let (send, recv) = oneshot::channel();
        let msg = SessionMessage::Healthy(send);
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor killed?")
    }

    pub async fn stdout(&self) -> Vec<String> {
        let (send, recv) = oneshot::channel();
        let msg = SessionMessage::Stdout(send);
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor killed?")
    }

    pub async fn stderr(&self) -> Vec<String> {
        let (send, recv) = oneshot::channel();
        let msg = SessionMessage::Stderr(send);
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor killed?")
    }
}
