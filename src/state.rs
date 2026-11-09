use std::path::PathBuf;
use std::process::Child;

// #[derive(Debug)]
// pub enum JobStatus {
//     Running,
//     Stopped,
//     Done,
// }

// #[derive(Debug)]
// pub struct Job {
//     pub id: usize,
//     pub pid: u32,
//     pub command: String,
//     pub status: JobStatus,
// }

// #[derive(Debug)]
// pub struct BackgroundProcess {
//     pub pid: u32,
//     pub command: String,
//     pub child: Child,
// }

#[derive(Debug)]
pub struct ShellState {
    pub cwd: PathBuf,
    pub previous_directory: Option<PathBuf>,
    /// Exit status of the most recent foreground command.
    pub last_status: i32,
    /// Set by `exit`. The shell stops and exits with this code.
    pub exit_requested: Option<i32>,
    /// Background pipelines that are still running.
    pub background: Vec<Child>,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            previous_directory: None,
            last_status: 0,
            exit_requested: None,
            background: Vec::new(),
        }
    }
}