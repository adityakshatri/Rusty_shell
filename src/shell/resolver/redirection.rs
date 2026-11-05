use crate::shell::parser::{Redirect, RedirectMode};

use std::fs::{File, OpenOptions};

#[derive(Debug, Default)]
pub struct ResolvedRedirections {
    pub stdin: Option<File>,
    pub stdout: Option<File>,
}

impl ResolvedRedirections {
    pub fn is_empty(&self) -> bool {
        self.stdin.is_none() && self.stdout.is_none()
    }
}

pub fn resolve(redirects: &[Redirect]) -> Result<ResolvedRedirections, String> {
    let mut resolved = ResolvedRedirections::default();

    // Process redirects in source order.
    for redirect in redirects {
        match &redirect.mode {
            RedirectMode::Input => {
                let file = File::open(&redirect.target)
                    .map_err(|e| format!("{}: {}", redirect.target, e))?;

                resolved.stdin = Some(file);
            }

            RedirectMode::Output => {
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&redirect.target)
                    .map_err(|e| format!("{}: {}", redirect.target, e))?;

                resolved.stdout = Some(file);
            }

            RedirectMode::Append => {
                let file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(true)
                    .open(&redirect.target)
                    .map_err(|e| format!("{}: {}", redirect.target, e))?;

                resolved.stdout = Some(file);
            }
        }
    }

    Ok(resolved)
}