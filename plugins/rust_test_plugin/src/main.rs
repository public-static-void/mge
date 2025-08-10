//! Rust test plugin

use serde::{Deserialize, Serialize};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;

/// Plugin requests
#[derive(Debug, Serialize, Deserialize)]
pub enum PluginRequest {
    /// Initialize plugin
    Initialize,
    /// Reload plugin
    Reload,
    /// Shutdown plugin
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
        /// Command result
        result: serde_json::Value,
    },
    /// Error
    Error {
        /// Error message
        message: String,
    },
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: plugin <socket_path>");
        std::process::exit(1);
    }
    let socket_path = &args[1];
    let stream = UnixStream::connect(socket_path)?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut writer = stream;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let req: PluginRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err = PluginResponse::Error {
                    message: e.to_string(),
                };
                let msg = serde_json::to_string(&err)? + "\n";
                writer.write_all(msg.as_bytes())?;
                continue;
            }
        };
        let resp = match req {
            PluginRequest::Initialize => {
                // Initialize plugin state here
                PluginResponse::Initialized
            }
            PluginRequest::Reload => {
                // Reload plugin state here
                PluginResponse::Reloaded
            }
            PluginRequest::Shutdown => PluginResponse::Shutdown,
            PluginRequest::RunCommand { data, .. } => {
                // Handle custom commands here
                PluginResponse::CommandResult {
                    result: serde_json::json!({"echo": data}),
                }
            }
        };
        let msg = serde_json::to_string(&resp)? + "\n";
        writer.write_all(msg.as_bytes())?;
        if matches!(resp, PluginResponse::Shutdown) {
            break;
        }
    }
    Ok(())
}
