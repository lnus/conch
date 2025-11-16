use anyhow::Result;
use nu_ansi_term::{Color, Style};
use std::path::{Path, PathBuf};

struct Segment {
    text: String,
    style: Style,
}

struct Prompt {
    segments: Vec<Segment>,
}

impl Prompt {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    fn push(&mut self, text: impl Into<String>, style: Style) {
        self.segments.push(Segment {
            text: text.into(),
            style,
        })
    }

    fn push_if(&mut self, text: Option<String>, style: Style) {
        if let Some(text) = text {
            self.push(text, style);
        }
    }

    fn print(&self) {
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                print!(" ");
            }
            print!("{}", seg.style.paint(&seg.text));
        }
    }
}

struct GitContext {
    branch: Option<String>,
    root: Option<PathBuf>,
    dirty: bool,
}

impl GitContext {
    fn discover(cwd: &Path) -> Option<Self> {
        let repo = gix::discover(cwd).ok()?;

        let branch = repo.head().ok().and_then(|head| {
            let name = head.referent_name()?;
            Some(name.shorten().to_string())
        });

        let root = repo.workdir().and_then(|wd| wd.canonicalize().ok());
        let dirty = repo.is_dirty().unwrap_or(false);

        Some(Self {
            branch,
            root,
            dirty,
        })
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

fn git_relative_path(cwd: &Path, git: Option<&GitContext>) -> Option<String> {
    let git = git?;
    let root = git.root.as_ref()?;

    let relative = cwd.strip_prefix(root).ok()?;
    let name = root.file_name()?.to_str().unwrap_or("?how?");

    if relative.as_os_str().is_empty() {
        Some(name.to_string())
    } else {
        Some(format!("{}/{}", name, relative.display()))
    }
}

fn home_relative_path(cwd: &Path) -> Option<String> {
    let home = dirs::home_dir()?;

    if cwd == home {
        return Some("~".to_string());
    }

    let relative = cwd.strip_prefix(&home).ok()?;
    Some(format!("~/{}", abbreviate_path(relative)))
}

// maybe inline these abstractions (home_rel*, git_rel*)
fn format_path(cwd: &Path, git: Option<&GitContext>) -> String {
    git_relative_path(cwd, git)
        .or_else(|| home_relative_path(cwd))
        .unwrap_or_else(|| cwd.display().to_string())
}

fn format_git(git: &GitContext) -> Option<String> {
    let mut result = git.branch.as_ref()?.clone();
    if git.dirty {
        result.push('*');
    }
    Some(result)
}

fn main() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let git = GitContext::discover(&cwd);

    let mut prompt = Prompt::new();

    prompt.push(
        format_path(&cwd, git.as_ref()),
        Style::new().fg(Color::Cyan).bold(),
    );

    prompt.push_if(
        git.as_ref().and_then(format_git),
        Style::new().fg(Color::Purple),
    );

    prompt.push_if(
        std::env::var("IN_NIX_SHELL")
            .ok()
            .map(|_| "nix".to_string()),
        Style::new().fg(Color::Yellow),
    );

    prompt.print();

    Ok(())
}
