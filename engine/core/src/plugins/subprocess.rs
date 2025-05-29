use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub enum PluginRequest {
    Initialize,
    Reload,
    Shutdown,
    RunCommand {
        command: String,
        data: serde_json::Value,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PluginResponse {
    Initialized,
    Reloaded,
    Shutdown,
    CommandResult { result: serde_json::Value },
    Error { message: String },
}

pub struct PluginSubprocess {
    child: Child,
    stream: UnixStream,
    socket_path: String,
}

impl PluginSubprocess {
    pub fn spawn<P: AsRef<std::path::Path>>(
        bin_path: P,
        socket_path: &str,
    ) -> Result<Self, String> {
        // Remove existing socket if present
        let _ = fs::remove_file(socket_path);

        // Start listener first so plugin can connect
        let listener =
            UnixListener::bind(socket_path).map_err(|e| format!("Failed to bind socket: {e}"))?;

        let child = Command::new(bin_path.as_ref())
            .arg(socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn plugin: {e}"))?;

        // Accept connection (with timeout)
        let stream = loop {
            match listener.accept() {
                Ok((stream, _addr)) => break stream,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    } else {
                        return Err(format!("Socket accept error: {e}"));
                    }
                }
            }
        };

        Ok(Self {
            child,
            stream,
            socket_path: socket_path.to_string(),
        })
    }

    pub fn send_request(&mut self, request: &PluginRequest) -> Result<PluginResponse, String> {
        let msg = serde_json::to_string(request).map_err(|e| e.to_string())? + "\n";
        self.stream
            .write_all(msg.as_bytes())
            .map_err(|e| e.to_string())?;
        self.stream.flush().map_err(|e| e.to_string())?;

        let mut reader = BufReader::new(&self.stream);
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| e.to_string())?;
        serde_json::from_str(&line).map_err(|e| e.to_string())
    }

    pub fn terminate(&mut self) {
        let _ = self.child.kill();
        let _ = fs::remove_file(&self.socket_path);
    }
}

impl Drop for PluginSubprocess {
    fn drop(&mut self) {
        self.terminate();
    }
}
