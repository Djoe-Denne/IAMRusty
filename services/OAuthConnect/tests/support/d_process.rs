//! Owned Windows child with separate bounded pipe drains. No stdout/stderr dumps.
//! Kill only this child as last resort; no claim of tested Windows signal shutdown.
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const LIMIT: usize = 128 * 1024;
enum Event {
    Bytes(usize, Vec<u8>),
    Done(usize, bool),
}
fn drain(
    mut pipe: impl Read + Send + 'static,
    index: usize,
    events: SyncSender<Event>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 2048];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) => {
                    let _ = events.send(Event::Done(index, true));
                    break;
                }
                Ok(count) => {
                    if events
                        .send(Event::Bytes(index, buffer[..count].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    let _ = events.send(Event::Done(index, false));
                    break;
                }
            }
        }
    })
}

pub struct Outcome {
    pub success: bool,
    text: String,
}
impl Outcome {
    pub fn contains(&self, marker: &str) -> bool {
        self.text.contains(marker)
    }
}

pub struct Process {
    child: Option<Child>,
    receiver: Receiver<Event>,
    drains: Vec<JoinHandle<()>>,
    buffers: [Vec<u8>; 2],
    done: [bool; 2],
    healthy: bool,
    exited: bool,
    success: bool,
}
impl Process {
    pub fn spawn(
        executable: &Path,
        cwd: &Path,
        args: &[&str],
        marker: Option<(&str, &str)>,
    ) -> Self {
        let mut command = Command::new(executable);
        command
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        for name in ["SystemRoot", "WINDIR", "PATH", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command.env("RUN_ENV", "test").env("NO_COLOR", "1");
        if let Some((name, value)) = marker {
            command.env(name, value);
        }
        let mut child = command
            .spawn()
            .expect("requested executable must exist and spawn; failure is NOT product RED");
        let stdout = child.stdout.take().expect("owned stdout pipe");
        let stderr = child.stderr.take().expect("owned stderr pipe");
        let (tx, receiver) = mpsc::sync_channel(16);
        let drains = vec![drain(stdout, 0, tx.clone()), drain(stderr, 1, tx)];
        Self {
            child: Some(child),
            receiver,
            drains,
            buffers: [Vec::new(), Vec::new()],
            done: [false; 2],
            healthy: true,
            exited: false,
            success: false,
        }
    }
    fn tick(&mut self, remaining: Duration) {
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.exited = true;
                    self.success = status.success();
                }
                Ok(None) => {}
                Err(_) => self.healthy = false,
            }
        }
        match self
            .receiver
            .recv_timeout(remaining.min(Duration::from_millis(25)))
        {
            Ok(Event::Bytes(index, bytes)) => {
                if bytes.len() <= LIMIT - self.buffers[index].len() {
                    self.buffers[index].extend(bytes);
                } else {
                    self.healthy = false;
                }
            }
            Ok(Event::Done(index, healthy)) => {
                self.done[index] = true;
                self.healthy &= healthy;
            }
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
    }
    fn text(&self) -> String {
        format!(
            "{}\n{}",
            String::from_utf8_lossy(&self.buffers[0]),
            String::from_utf8_lossy(&self.buffers[1])
        )
    }
    /// Wait for an observable bounded log marker/address, never print the supplied text.
    pub fn observe_until(
        &mut self,
        timeout: Duration,
        mut predicate: impl FnMut(&str) -> bool,
    ) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if predicate(&self.text()) {
                return true;
            }
            if Instant::now() >= deadline || (self.exited && self.done.iter().all(|d| *d)) {
                return false;
            }
            self.tick(deadline.saturating_duration_since(Instant::now()));
        }
    }
    fn stop(&mut self) -> bool {
        if !self.exited {
            if let Some(child) = &mut self.child {
                let _ = child.kill();
            }
        }
        let deadline = Instant::now() + Duration::from_secs(4);
        while Instant::now() < deadline && !(self.exited && self.done.iter().all(|d| *d)) {
            self.tick(deadline.saturating_duration_since(Instant::now()));
        }
        let complete = self.exited && self.done.iter().all(|d| *d);
        if complete {
            // try_wait has already observed/reaped this child. No unbounded blocking wait.
            if let Some(mut child) = self.child.take() {
                self.healthy &= child.wait().is_ok();
            }
            for handle in self.drains.drain(..) {
                self.healthy &= handle.join().is_ok();
            }
        }
        complete
    }
    /// Always finish owned process/pipes before returning evidence or failing assertions.
    pub fn finish(&mut self, timeout: Duration) -> Outcome {
        let _ = self.observe_until(timeout, |_| false);
        let natural_success = self.exited && self.success;
        let closed = self.stop();
        let text = self.text();
        assert!(closed, "owned child/pipes did not stop within deadline; cleanup requires parent reconciliation");
        assert!(
            self.healthy,
            "child pipe failed or exceeded bounded output capture"
        );
        for marker in [
            "sentinel",
            "PRIVATE KEY",
            "CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
            "eyJhbGci",
            "one@example.test",
        ] {
            assert!(
                !text.contains(marker),
                "child diagnostic exposed confidential content"
            );
        }
        Outcome {
            success: natural_success,
            text,
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
