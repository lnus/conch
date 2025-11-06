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
    fn gather() -> Option<Self> {
        let branch = get_branch_name_sh().ok()?;
        if branch.is_empty() {
            return None;
        }

        let root = get_repo_root_sh().ok()?;
        let dirty = get_repo_status_sh().unwrap_or(false);
        let (behind, ahead) = get_ahead_behind_sh().unwrap_or((0, 0));

        Some(Self {
            branch,
            root,
            dirty,
            ahead,
            behind,
        })
    }

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

fn abbreviate_path(path: &Path) -> String {
    let components: Vec<_> = path.components().collect();

    if components.len() <= 1 {
        return path.display().to_string();
    }

    let abbreviated: Vec<String> = components
        .iter()
        .take(components.len() - 1)
        .filter_map(|c| {
            c.as_os_str()
                .to_str()
                .and_then(|s| s.chars().next())
                .map(|ch| ch.to_string())
        })
        .collect();

    let last = components.last().unwrap().as_os_str().to_string_lossy();

    format!("{}/{}", abbreviated.join("/"), last)
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

// TODO PERF: consider doing this from file traversal?
// TODO PERF: cwd->repo cache?
fn get_branch_name_sh() -> Result<String> {
    // NOTE: since git 2.22 `git branch --show-current` works
    let output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()?;
    let s = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(s)
}

fn get_repo_root_sh() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(PathBuf::from(path))
}

fn get_repo_status_sh() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;

    Ok(!output.stdout.is_empty())
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
