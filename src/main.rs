use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::fs::{File};
use daemonize::Daemonize; 
use std::{time::Duration};
use tokio::{
    process::Command,
    fs::{OpenOptions},
};
use tracing;

mod ipc;
mod utils;
mod application;

use ipc::listen_socket;
use application::Application;

fn load_config() -> Result<Vec<Application>, Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap();
    let config_file_path = format!("{}/rust_daemon/config.json", home);
    //let config_file_path = "/rust_daemon/config.json";
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
                println!("Starting app - {} ", app_clone.name);
                let stdout = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&app_clone.stdout_logfile)
                        .await
                        .unwrap_or_else(|e| panic!("Failed to open stdout: {}", e));

                let stderr = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&app_clone.stderr_logfile)
                        .await
                        .unwrap_or_else(|e| panic!("Failed to open stderr: {}", e));

                let mut cmd = match Command::new(&app_clone.command)
                        .current_dir(&app_clone.working_dir)
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

                // Monitor process
                let _result = cmd.wait().await;
                let mut map = pids_map.lock().await;
                map.remove(&app_clone.name);
                if let Err(e) = cmd.wait().await {
                    tracing::error!("Process failed: {}", e);
                }

                // Wait before restarting
                tokio::time::sleep(Duration::from_secs(1)).await;
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
    tokio::spawn(async {
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
    let working_dir = format!("{}/rust_daemon/", home);
    let stdout_path = format!("{}/rust_daemon/daemon.out", home);
    let stderr_path = format!("{}/rust_daemon/daemon.err", home);

    //let working_dir = "/rust_daemon/";
    //let stdout_path = "/rust_daemon/daemon.out";
    //let stderr_path = "/rust_daemon/daemon.err";
    
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
