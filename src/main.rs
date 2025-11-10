use anyhow::Result;
use nu_ansi_term::Color;
use std::path::{Path, PathBuf};

trait PromptSegment {
    fn display(&self) -> Option<String>;
}

impl<T: PromptSegment> PromptSegment for Option<T> {
    fn display(&self) -> Option<String> {
        self.as_ref().and_then(|s| s.display())
    }
}

#[derive(Debug, Default)]
struct GitInfo {
    branch: Option<String>,
    root: Option<PathBuf>,
    dirty: bool,
}

// TODO: add back better ahead/behind
impl GitInfo {
    fn gather(cwd: &Path) -> Self {
        let Ok(repo) = gix::discover(cwd) else {
            return Self::default();
        };

        let branch = repo.head().ok().and_then(|head| {
            let name = head.referent_name()?;
            Some(name.shorten().to_string())
        });

        let root = repo.workdir().and_then(|wd| wd.canonicalize().ok());
        let dirty = repo.is_dirty().unwrap_or(false);

        Self {
            branch,
            root,
            dirty,
        }
    }
}

impl PromptSegment for GitInfo {
    fn display(&self) -> Option<String> {
        let mut result = self.branch.as_ref()?.clone();

        if self.dirty {
            result.push_str("*");
        }

        Some(Color::Purple.paint(result).to_string())
    }
}

struct PathInfo {
    cwd: PathBuf,
    git_root: Option<PathBuf>,
}

impl PathInfo {
    fn new(cwd: PathBuf, git_root: Option<PathBuf>) -> Self {
        Self { cwd, git_root }
    }

    fn build_display(&self) -> String {
        self.git_relative_path()
            .or_else(|| self.home_relative_path())
            .unwrap_or_else(|| self.cwd.display().to_string())
    }

    fn git_relative_path(&self) -> Option<String> {
        let repo = self.git_root.as_ref()?;
        let relative = self.cwd.strip_prefix(repo).ok()?;

        let name = repo.file_name()?.to_str().unwrap_or("?how?");

        if relative.as_os_str().is_empty() {
            Some(name.to_string())
        } else {
            Some(format!("{}/{}", name, relative.display()))
        }
    }

    fn home_relative_path(&self) -> Option<String> {
        let home = dirs::home_dir()?;

        if self.cwd == home {
            return Some("~".to_string());
        }

        let relative = self.cwd.strip_prefix(&home).ok()?;
        Some(format!("~/{}", abbreviate_path(relative)))
    }
}

impl PromptSegment for PathInfo {
    fn display(&self) -> Option<String> {
        Some(Color::Cyan.bold().paint(self.build_display()).to_string())
    }
}

struct EnvIndicator {
    key: &'static str,
    display: &'static str,
}

impl EnvIndicator {
    fn new(key: &'static str, display: &'static str) -> Self {
        Self { key, display }
    }
}

impl PromptSegment for EnvIndicator {
    fn display(&self) -> Option<String> {
        std::env::var(self.key).ok()?;
        Some(Color::Yellow.paint(self.display).to_string())
    }
}

struct Prompt {
    parts: Vec<String>,
}

impl Prompt {
    fn new() -> Self {
        Self { parts: Vec::new() }
    }

    fn add_if(mut self, segment: impl PromptSegment) -> Self {
        if let Some(display) = segment.display() {
            self.parts.push(display);
        }
        self
    }

    fn build(&self) -> String {
        self.parts.join(" ")
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

fn main() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let git_info = GitInfo::gather(&cwd);

    let prompt = Prompt::new()
        .add_if(PathInfo::new(cwd, git_info.root.clone()))
        .add_if(git_info)
        .add_if(EnvIndicator::new("IN_NIX_SHELL", "nix"));

    print!("{}", prompt.build());

    Ok(())
}
