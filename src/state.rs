//states of the current running shell process
use std::collections::{HashMap, HashSet};
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

    vars: HashMap<String, String>,
    exported_vars: HashSet<String>,
}

impl ShellState {
    pub fn new() -> Self {
            let mut vars = HashMap::new();
            let mut exported_vars = HashSet::new();

            // Initialize environment variables from the current process
            for (key, value) in std::env::vars_os(){
                let key = key.to_string_lossy().into_owned();
                vars.insert(key.clone(), value.to_string_lossy().into_owned());
                exported_vars.insert(key);
            }

            Self {
                cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
                previous_directory: None,
                last_status: 0,
                exit_requested: None,
                background: Vec::new(),
                vars,
                exported_vars,
            }
    }

    pub fn get_var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(String::as_str)
    }

    pub fn set_var(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.vars.insert(name.into(), value.into());
    }

    pub fn export_var(&mut self, name:&str, value: Option<String>) {
        if let Some(value) = value {
            self.vars.insert(name.to_string(), value);
        }
        self.exported_vars.insert(name.to_string());
    }

    pub fn unset_var(&mut self, name: &str) {
        self.vars.remove(name);
        self.exported_vars.remove(name);
    }

    pub fn var_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.vars.keys().cloned().collect();
        names.sort();
        names
    }

    //what this does in detail is it returns a vector of tuples containing the names and values of all exported environment variables. It iterates over the set of exported variable names, retrieves their corresponding values from the vars HashMap, and collects them into a vector. This allows the shell to easily access and display the current environment variables that are marked for export.
    //collect works by taking an iterator and transforming it into a collection, in this case, a vector of tuples. Each tuple contains the name and value of an exported variable, making it convenient to work with the environment variables in the shell.
    pub fn exported_env(&self) ->Vec<(String, String)> {
        self.exported_vars.iter().filter_map(|name| {
            self.vars.get(name).map(|value| (name.clone(), value.clone()))
        }).collect()
    }

}

pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
