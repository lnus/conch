use nu_ansi_term::Color::*;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;

#[derive(Debug)]
struct GitInfo {
    branch: String,
    root: PathBuf,
    dirty: bool,
    ahead: usize,
    behind: usize,
}

impl GitInfo {
    // TODO: split out and don't insta fail into propogate None always
    // for now this works fine though, we could just unwrap some stuff into defaults
    fn gather() -> Option<Self> {
        let repo = gix::discover(".").ok()?;
        let head = repo.head().ok()?;
        let branch = head
            .referent_name()
            .and_then(|n| n.shorten().to_string().into())?;

        // FIX this line is kinda ugly
        // workdir is rel. path, but surely we can get canonical path easier?
        let root = repo.workdir()?.to_path_buf().canonicalize().ok()?;

        // could probably do more with status rather than just dirty y||n
        // we're already pulling the entire dep
        let dirty = repo.is_dirty().ok()?;

        // TODO: rewrite to gix, too lazy rn.
        // or remove tbh i don't care about this too much
        let (behind, ahead) = get_ahead_behind_sh().unwrap_or((0, 0));

        Some(Self {
            branch,
            root,
            dirty,
            ahead,
            behind,
        })
    }

    // reconsider
    fn format(&self) -> String {
        let mut result = self.branch.clone();

        if self.dirty {
            result.push_str("*");
        }

        let mut indicators = vec![];
        if self.ahead > 0 {
            indicators.push(format!("{} ahead", self.ahead));
        }
        if self.behind > 0 {
            indicators.push(format!("{} behind", self.behind));
        }

        if !indicators.is_empty() {
            result.push(' ');
            result.push_str(&indicators.join(" "));
        }

        result
    }
}

// magnum opus function
fn abbreviate_path(path: &Path) -> String {
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

fn build_path_display(cwd: &Path, repo: Option<&Path>) -> Result<String> {
    if let Some(repo) = repo {
        if let Ok(relative) = cwd.strip_prefix(repo) {
            let name = repo.file_name().and_then(|n| n.to_str()).unwrap_or("?how?");

            if relative.as_os_str().is_empty() {
                return Ok(name.to_string());
            }

            return Ok(format!("{}/{}", name, relative.display()));
        }
    }

    if let Some(home) = dirs::home_dir() {
        if cwd == home {
            return Ok("~".to_string());
        }

        if let Ok(relative) = cwd.strip_prefix(&home) {
            return Ok(format!("~/{}", abbreviate_path(&relative)));
        }
    }

    Ok(cwd.display().to_string())
}

fn get_ahead_behind_sh() -> Result<(usize, usize)> {
    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        .output()?;

    let s = String::from_utf8(output.stdout)?;
    let mut parts = s.trim().split_whitespace();

    let behind = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
    let ahead = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);

    Ok((behind, ahead))
}

fn main() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let git_info = GitInfo::gather();

    let path = build_path_display(&cwd, git_info.as_ref().map(|g| g.root.as_path()))?;

    print!("{}", Cyan.bold().paint(path));

    if let Some(info) = git_info {
        print!(" {}", Purple.paint(info.format()));
    }

    Ok(())
}
