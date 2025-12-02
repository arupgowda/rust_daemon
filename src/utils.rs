use serde::{Serialize};
use procfs::{process};
use std::{
    sync::Arc,
    path::Path,
    collections::HashMap,
};
use tokio::{
    process::Command,
    fs::{OpenOptions},
    sync::Mutex,
};
use serde_json::{Value, json};
use shellexpand::tilde;

#[derive(Serialize)]
struct Status {
    app: String,
    uptime: f64,
}

use crate::application::Application;

// Get stats about all running applications
pub async fn get_stats(pids_map: Arc<Mutex<HashMap<String, u32>>>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {

    let mut statuses = Vec::new();

    let app_pids: Vec<(String, u32)> = {
        let map = pids_map.lock().await;
        map.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }; // release lock

    // process status
    for (app, pid) in app_pids {
        let process = process::Process::new(pid as i32)?;
        let stat = process.stat()?;

        // Get system uptime
        let system_uptime_seconds = sysinfo::System::uptime() as f64;

        // Get process start time in clock ticks
        let process_start_time_ticks = stat.starttime as f64;

        // Get clock ticks per second
        let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) as f64 };

        // Calculate process start time in seconds since boot
        let process_start_time_seconds = process_start_time_ticks / ticks_per_second;

        // Calculate running time
        let running_time_seconds = system_uptime_seconds - process_start_time_seconds;

        println!("App {} with process PID {} has been running for {:.2} seconds", app, pid, running_time_seconds);

        statuses.push(Status {
            app: app.clone(),
            uptime: running_time_seconds,
        });
    }

    Ok(json!({
        "statuses": statuses,
        "count": statuses.len()
       }))
}

// Start an application
pub async fn start_app(pids_map: Arc<Mutex<HashMap<String, u32>>>, apps: Arc<Vec<Application>>, app_name: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>>
{
    // First check if App is valid
    let app = apps.iter()
        .find(|a| a.name == app_name)
        .ok_or("Application not found")?;

    // Check if app is already running
    let is_running = {
        let map = pids_map.lock().await;
        map.contains_key(&app_name)
    }; // lock released

    if is_running {
        return Ok("Application is already running".to_string());
    }
    else {
        println!("Starting app - {}", app_name);

        let stdout_path = tilde(&app.stdout_logfile).into_owned();
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(&stdout_path))
            .await
            .unwrap_or_else(|e| panic!("Failed to open stdout: {}", e));

        let stderr_path = tilde(&app.stderr_logfile).into_owned();
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(Path::new(&stderr_path))
            .await
            .unwrap_or_else(|e| panic!("Failed to open stderr: {}", e));

        let command_path = tilde(&app.command).into_owned();
        let working_dir_path = tilde(&app.working_dir).into_owned();
        match Command::new(Path::new(&command_path))
            .current_dir(Path::new(&working_dir_path))
            .stdout(stdout.into_std().await)
            .stderr(stderr.into_std().await)
            .kill_on_drop(true)
            .spawn() // this starts the application immediately
            {
                Ok(mut child) => {
                    let child_id = child.id().unwrap_or(0);
                    println!("Application {} started with pid {}", app.name, child_id);
                    let mut map = pids_map.lock().await;
                    map.insert(app.name.clone(), child.id().unwrap());

                    let app_name_clone = app.name.clone();
                    let pids_map_clone = Arc::clone(&pids_map);

                    // Move child into tokio task to monitor
                    tokio::spawn(async move {
                        // Monitor process for termination
                        child.wait().await.ok();
                        let mut map = pids_map_clone.lock().await;
                        map.remove(&app_name_clone);
                    });

                    return Ok(format!("Application {} started with pid {}", app.name, child_id));
                },
                Err(e) => {
                    return Err(format!("Application failed to start successfully with error {}", e).into()); 
                }
            };
    }
}
