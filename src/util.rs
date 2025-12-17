use std::{path::Path, time::Duration};

use crate::repo::RepoContext;

// magnum opus function
#[must_use]
pub fn abbreviate_path(path: &Path) -> String {
    match path.components().collect::<Vec<_>>().as_slice() {
        [] | [_] => path.display().to_string(),
        [init @ .., last] => {
            let abbreviated = init
                .iter()
                .filter_map(|c| c.as_os_str().to_str()?.chars().next())
                .fold(String::new(), |mut acc, ch| {
                    if !acc.is_empty() {
                        acc.push('/');
                    }
                    acc.push(ch);
                    acc
                });

            format!("{}/{}", abbreviated, last.as_os_str().to_string_lossy())
        }
    }
}

#[must_use]
pub fn format_path(cwd: &Path, repo: Option<&RepoContext>) -> String {
    repo.and_then(|repo| {
        let relative = cwd.strip_prefix(repo.root()).ok()?;
        let name = repo.root().file_name()?.to_str().unwrap_or("?how?");

        if relative.as_os_str().is_empty() {
            Some(name.to_string())
        } else {
            Some(format!("{}/{}", name, relative.display()))
        }
    })
    .or_else(|| {
        let home = dirs::home_dir()?;

        if cwd == home {
            return Some("~".to_string());
        }

        let relative = cwd.strip_prefix(&home).ok()?;
        Some(format!("~/{}", abbreviate_path(relative)))
    })
    .unwrap_or_else(|| cwd.display().to_string())
}

#[must_use]
pub fn format_repo(repo: &RepoContext) -> String {
    let mut reference = repo.reference();
    if repo.dirty() {
        reference.push('*');
    }
    reference
}

#[must_use]
pub fn format_duration(duration: Duration) -> Option<String> {
    match duration.as_millis() {
        0..100 => None,
        100..1000 => Some(format!("{}ms", duration.as_millis())),
        1000..60_000 => Some(format!("{:.1}s", duration.as_secs_f64())),
        60_000..3_600_000 => {
            let secs = duration.as_secs();
            Some(format!("{}m{}s", secs / 60, secs % 60))
        }
        _ => {
            let secs = duration.as_secs();
            Some(format!("{}h{}m", secs / 3600, (secs % 3600) / 60))
        }
    }
}
