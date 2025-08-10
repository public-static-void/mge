use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// Plugin requests
#[derive(Debug, Serialize, Deserialize)]
pub enum PluginRequest {
    /// Initialize
    Initialize,
    /// Reload
    Reload,
    /// Shutdown
    Shutdown,
    /// Run command
    RunCommand {
        /// Command
        command: String,
        /// Arguments
        data: serde_json::Value,
    },
}

/// Plugin responses
#[derive(Debug, Serialize, Deserialize)]
pub enum PluginResponse {
    /// Plugin initialized
    Initialized,
    /// Plugin reloaded
    Reloaded,
    /// Plugin shutdown
    Shutdown,
    /// Command result
    CommandResult {
        /// Result
        result: serde_json::Value,
    },
    /// Error
    Error {
        /// Error message
        message: String,
    },
}

/// Plugin subprocess
pub struct PluginSubprocess {
    child: Child,
    stream: UnixStream,
    socket_path: String,
}

impl PluginSubprocess {
    /// Spawn plugin subprocess
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

    /// Send request
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

    /// Terminate
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
