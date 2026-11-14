use std::fs::{File, OpenOptions};

use crate::shell::expand::ExpandedRedirect;
use crate::shell::parser::RedirectMode;

#[derive(Debug, Default)]
pub struct ResolvedRedirections {
    pub stdin: Option<File>,
    pub stdout: Option<File>,
}

/// Opens every redirect file in source order. Later redirects replace earlier ones.
pub fn resolve(redirects: &[ExpandedRedirect]) -> Result<ResolvedRedirections, String> {
    let mut resolved = ResolvedRedirections::default();

    for redirect in redirects {
        match redirect.mode {
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