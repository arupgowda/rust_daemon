use std::collections::HashMap;
use std::sync::Arc;
use std::fs::{File};
use daemonize::Daemonize; 
use std::{
    time::Duration,
    path::Path,
};
use tracing;
use shellexpand::tilde;
use tokio::{
    process::Command,
    fs::{OpenOptions},
    sync::Mutex,
};

mod ipc;
mod utils;
mod application;

use ipc::listen_socket;
use application::Application;

fn load_config() -> Result<Vec<Application>, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap();
    let config_file_path = format!("{}/src/rust_daemon/config.json", home);
    let file = File::open(config_file_path)?;
    let apps:Vec<Application> = serde_json::from_reader(file).expect("JSON improperly formatted");
    //println!("{:?}",apps);
    Ok(apps)
}

async fn run_async_processes(apps: Arc<Vec<Application>>, pids_map: Arc<Mutex<HashMap<String, u32>>>) -> tokio::io::Result<()> {
    for app in apps.iter().filter(|a| a.auto_start) {
        let app_clone = app.clone();
        let pids_map = Arc::clone(&pids_map);

        tokio::spawn(async move {
            loop { 
                println!("Starting app - {}", app_clone.name);

                // Expands ~ in config file and returns an expanded owned string
                let stdout_path = tilde(&app_clone.stdout_logfile).into_owned();
                let stdout = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(Path::new(&stdout_path)) // Path::new turns a string into a
                                                       // FileSystem type
                        .await
                        .unwrap_or_else(|e| panic!("Failed to open stdout: {}", e));

                let stderr_path = tilde(&app_clone.stderr_logfile).into_owned();
                let stderr = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(Path::new(&stderr_path))
                        .await
                        .unwrap_or_else(|e| panic!("Failed to open stderr: {}", e));

                let command_path = tilde(&app_clone.command).into_owned();
                let working_dir_path = tilde(&app_clone.working_dir).into_owned();
                let mut cmd = match Command::new(Path::new(&command_path))
                        .current_dir(Path::new(&working_dir_path))
                        .stdout(stdout.into_std().await)
                        .stderr(stderr.into_std().await)
                        .kill_on_drop(true)
                        .spawn()
                {
                    Ok(child) => {
                        println!("Spawned process {}", child.id().unwrap_or(0));
                        let mut map = pids_map.lock().await;
                        map.insert(app_clone.name.clone(), child.id().unwrap());
                        child
                    },
                    Err(e) => {
                        tracing::error!(
                            "Failed to spawn '{}': {}",
                            app_clone.command,
                            e
                        );
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                // Monitor process for termination
                if let Err(e) = cmd.wait().await {
                    tracing::error!("Process failed: {}", e);
                }

                // Remove PID from hash map
                let mut map = pids_map.lock().await;
                map.remove(&app_clone.name);

                // Wait before restarting
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }
    Ok(())
}

async fn async_main(apps: Vec<Application>) -> Result<(), Box<dyn std::error::Error>> {
    let pids_map = Arc::new(Mutex::new(HashMap::new()));
    let apps_arc = Arc::new(apps);

    run_async_processes(Arc::clone(&apps_arc), Arc::clone(&pids_map)).await?;

    // Spawn socket listener in background
    tokio::spawn(async move {
        if let Err(e) = listen_socket(pids_map, apps_arc).await {
            eprintln!("Socket listener failed: {}", e);
        }
    });

    // Keep daemon alive FOREVER
    futures::future::pending::<()>().await;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let apps: Vec<Application>;

    // Read config
    match load_config() {
        Ok(loaded_apps) => {
            apps = loaded_apps;
            println!("Found {} applications:", apps.len());
            for app in &apps {
                println!("- {} ", app.name);
            }
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    }

    let home = std::env::var("HOME").unwrap();
    let working_dir = format!("{}/src/rust_daemon/", home);
    let stdout_path = format!("{}/src/rust_daemon/daemon.out", home);
    let stderr_path = format!("{}/src/rust_daemon/daemon.err", home);

    let stdout = std::fs::File::create(&stdout_path).unwrap();
    let stderr = std::fs::File::create(&stderr_path)?;

    let daemonize = Daemonize::new()
        .pid_file("/tmp/daemon.pid")
        .chown_pid_file(true)
        .working_directory(working_dir)
        .stdout(stdout)
        .stderr(stderr);

    match daemonize.start() {
        Ok(_) => {
            println!("Daemon started (PID: {})", std::process::id());
            
            // START tokio runtime
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async_main(apps))?;
        }

        Err(e) => eprintln!("Daemon failed: {}",e),
    }
    Ok(())
}
