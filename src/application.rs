use serde::{Deserialize};

#[derive(Clone)] 
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Application {
    pub name: String,
    pub auto_start: bool,
    pub working_dir: String,
    pub command: String,
    pub stdout_logfile: String,
    pub stderr_logfile: String,
}

