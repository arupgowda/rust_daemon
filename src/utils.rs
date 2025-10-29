use serde::{Serialize};
use procfs::{process};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use serde_json::{Value, json};

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
pub async fn start_app(pids_map: Arc<Mutex<HashMap<String, u32>>>, apps: Arc<Vec<Application>>, app_name: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    // First check if App is valid
    let _app = apps.iter()
        .find(|a| a.name == app_name)
        .ok_or("Application not found")?;

    // Check is app is already running
    let is_running = {
        let map = pids_map.lock().await;
        map.contains_key(&app_name)
    }; // lock released

    if is_running {
        return Ok("Application is already running".to_string());
    }

    Ok("Application started successfully".to_string())
}
