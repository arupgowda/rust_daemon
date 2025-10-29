use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::net::{TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::utils::get_stats;
use crate::utils::start_app;
use crate::application::Application;

pub async fn listen_socket(pids_map: Arc<Mutex<HashMap<String, u32>>>, apps: Arc<Vec<Application>>) -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8081").await?;
    println!("Listening on 127.0.0.1:8081");
    loop {
        let (mut stream, addr) = listener.accept().await?;
        println!("New connection from: {}", addr);
        let tcp_nodelay = true;
        stream.set_nodelay(tcp_nodelay)?;
        let task_pids_map = Arc::clone(&pids_map);
        let task_apps = Arc::clone(&apps);

        // Handle each connection in its own task
        tokio::spawn(async move {
            let mut buf = [0u8; 512];

            match stream.read(&mut buf).await {
                Ok(n) if n == 0 => return,

                Ok(n) => {
                    let received = std::str::from_utf8(&buf[..n]).expect("valid utf8");
                    println!("raw: {}", received);
                    let parsed: serde_json::Value = serde_json::from_str(received).expect("Failed to parse JSON");
                    let app = parsed["app"].as_str().unwrap_or("unknown");

                    if parsed["command"] == "status" {
                        match get_stats(task_pids_map).await {
                            Ok(status_data) => {
                                let json_data = serde_json::to_string(&status_data).expect("Failed to convert to JSON");
                                 // Add newline for telnet
                                let response = format!("{}\n", json_data);
                                if let Err(e) = stream.write_all(response.as_bytes()).await {
                                    eprintln!("Write error: {}", e);
                                }
                                if let Err(e) = stream.flush().await {
                                    eprintln!("Flush error: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to get status: {}", e);
                            }
                        }
                    }
                    else if parsed["command"] == "start" {
                        let response = match start_app(task_pids_map, task_apps, app.to_string()).await {
                            Ok(result) => {
                                 // Add newline for telnet
                                format!("{}\n", result)
                            }
                            Err(e) => {
                                eprintln!("Failed to start: {}", e);
                                format!("Failed to start: {}\n", e)
                            }
                        };

                        if let Err(e) = stream.write_all(response.as_bytes()).await {
                            eprintln!("Write error: {}", e);
                        }
                        if let Err(e) = stream.flush().await {
                            eprintln!("Flush error: {}", e);
                        }
                    }
                },

                Err(e) => {
                    eprintln!("Failed to read from socket: {}", e);
                    return;
                }
            }
        });
    }
}
