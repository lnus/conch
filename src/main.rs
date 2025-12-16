use anyhow::Result;
use jj_lib::object_id::ObjectId;
use jj_lib::{
    repo::{Repo, StoreFactories},
    settings::UserSettings,
    workspace::{Workspace, default_working_copy_factories},
};
use nu_ansi_term::{Color, Style};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    time::Duration,
};

struct Segment {
    text: String,
    style: Style,
}

struct Prompt {
    segments: Vec<Segment>,
    separator: Option<String>,
    prefix: Option<String>,
    suffix: Option<String>,
    style: Style,
}

impl Prompt {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            separator: None,
            prefix: None,
            suffix: None,
            style: Style::default(),
        }
    }

    fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = Some(separator.into());
        self
    }

    fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    fn push(&mut self, text: impl Into<String>, style: Style) {
        self.segments.push(Segment {
            text: text.into(),
            style,
        });
    }

    fn push_if(&mut self, text: Option<String>, style: Style) {
        if let Some(text) = text {
            self.push(text, style);
        }
    }

    fn print(&self) {
        if let Some(prefix) = &self.prefix {
            print!("{}", self.style.paint(prefix));
        }

        let separator = self.separator.as_deref().unwrap_or(" ");

        self.segments.iter().enumerate().for_each(|(i, seg)| {
            if i > 0 {
                print!("{}", self.style.paint(separator));
            }
            print!("{}", seg.style.paint(&seg.text));
        });

        if let Some(suffix) = &self.suffix {
            print!("{}", self.style.paint(suffix));
        }
    }
}

#[derive(Debug)]
struct GitContext {
    branch: String,
    root: PathBuf,
    dirty: bool,
}

impl GitContext {
    fn discover(cwd: &Path) -> Option<Self> {
        let repo = gix::discover(cwd).ok()?;

        let branch = repo.head().ok().and_then(|head| {
            let name = head.referent_name()?;
            Some(name.shorten().to_string())
        })?;

        let root = repo.workdir().and_then(|wd| wd.canonicalize().ok())?;
        let dirty = repo.is_dirty().unwrap_or(false);

        Some(Self {
            branch,
            root,
            dirty,
        })
    }
}

#[derive(Debug)]
struct JujutsuContext {
    change_id: String,
    root: PathBuf,
    dirty: bool,
}

impl JujutsuContext {
    // TODO probably use jj-lib but too lazy now, quick hack
    #[allow(dead_code)]
    fn discover(cwd: &Path) -> Option<Self> {
        let mut check_path = cwd.to_path_buf();
        let root = loop {
            if check_path.join(".jj").exists() {
                break check_path;
            }
            check_path = check_path.parent()?.to_path_buf();
        };

        let change_id = std::process::Command::new("jj")
            .args([
                "log",
                "--color",
                "always",
                "--no-graph",
                "--limit",
                "1",
                "--revisions",
                "@",
                "-T",
                "change_id.shortest(4)",
            ])
            .current_dir(&root)
            .output()
            .ok()?
            .stdout;
        let change_id = String::from_utf8_lossy(&change_id).trim().to_string();

        let status = std::process::Command::new("jj")
            .args(["status"])
            .current_dir(&root)
            .output()
            .ok()?
            .stdout;

        let dirty = !String::from_utf8_lossy(&status).contains("no changes");

        Some(Self {
            change_id,
            root,
            dirty,
        })
    }

    #[allow(dead_code)]
    fn discover_jj_lib(cwd: &Path) -> Option<Self> {
        let user_settings =
            UserSettings::from_config(jj_lib::config::StackedConfig::with_defaults())
                .expect("Can't load settings"); // TODO: load actual settings, if necessary

        // TODO: proper error handling. We could just propagate none here v
        let workspace_root = cwd
            .ancestors()
            .find(|path| path.join(".jj").is_dir())
            .expect("No .jj directory found in parents");

        // TODO: proper error handling. We should do... something here.
        let store_factories = StoreFactories::default();
        let wc_factories = default_working_copy_factories();
        let workspace = Workspace::load(
            &user_settings,
            workspace_root,
            &store_factories,
            &wc_factories,
        )
        .expect("Can't load workspace");
        dbg!(workspace.workspace_root());
        dbg!(workspace.workspace_name().as_str());

        let repo = workspace
            .repo_loader()
            .load_at_head()
            .expect("Could not load repo head");

        let workspace_name = workspace.workspace_name();
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace_name)
            .expect("Could not get commit ID");

        let commit = repo
            .store()
            .get_commit(wc_commit_id)
            .expect("Could not resolve commit");

        dbg!(commit.id().hex());
        dbg!(commit.change_id().reverse_hex());

        let change_prefix_len = repo
            .shortest_unique_change_id_prefix_len(commit.change_id())
            .expect("Could not calculate shortest prefix");
        dbg!(change_prefix_len);

        let commit_prefix_len = repo
            .index()
            .shortest_unique_commit_id_prefix_len(commit.id())
            .expect("Could not calculate shortest prefix");
        dbg!(commit_prefix_len);

        // I think this is correct
        let discardable = commit
            .is_discardable(&*repo)
            .expect("Could not read is_discarable");
        dbg!(discardable);

        // FIX this is ugly, just gives some output
        // for now, just ignore.
        Some(Self {
            change_id: commit.change_id().reverse_hex()[..4].to_string(),
            root: workspace_root.to_path_buf(),
            dirty: !discardable,
        })
    }
}

#[derive(Debug)]
enum RepoContext {
    Git(GitContext),
    Jujutsu(JujutsuContext),
}

impl RepoContext {
    fn discover(cwd: &Path) -> Option<Self> {
        JujutsuContext::discover_jj_lib(cwd)
            .map(Self::Jujutsu)
            .or_else(|| GitContext::discover(cwd).map(Self::Git))
    }

    fn root(&self) -> &Path {
        match self {
            Self::Git(ctx) => &ctx.root,
            Self::Jujutsu(ctx) => &ctx.root,
        }
    }

    const fn dirty(&self) -> bool {
        match self {
            Self::Git(ctx) => ctx.dirty,
            Self::Jujutsu(ctx) => ctx.dirty,
        }
    }

    fn reference(&self) -> String {
        match self {
            Self::Git(ctx) => ctx.branch.clone(),
            Self::Jujutsu(ctx) => ctx.change_id.clone(),
        }
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

fn format_path(cwd: &Path, repo: Option<&RepoContext>) -> String {
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

fn format_repo(repo: &RepoContext) -> String {
    let mut result = repo.reference();
    if repo.dirty() {
        result.push('*');
    }
    result
}

fn format_duration(duration: Duration) -> Option<String> {
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

fn main() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo = RepoContext::discover(&cwd);

    let plain = std::env::var("CONCH_PLAIN")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    let multiline = std::env::var("CONCH_MULTILINE")
        .map(|v| v != "0" && v != "false")
        .unwrap_or(true);

    let mut prompt = match (plain, multiline) {
        (true, _) => Prompt::new(),
        (false, true) => Prompt::new()
            .with_separator(" ∵ ")
            .with_prefix("┏━ ")
            .with_suffix("\n┃"),
        (false, false) => Prompt::new().with_separator(" ∵ "),
    }
    .with_style(Style::new().fg(Color::Yellow));

    prompt.push(
        format_path(&cwd, repo.as_ref()),
        Style::new().fg(Color::Cyan).bold(),
    );

    prompt.push_if(
        repo.as_ref().map(format_repo),
        Style::new().fg(Color::Purple),
    );

    prompt.push_if(
        std::env::var("IN_NIX_SHELL")
            .ok()
            .map(|_| "nix".to_string()),
        Style::new().fg(Color::LightBlue),
    );

    prompt.push_if(
        std::env::var("DIRENV_FILE")
            .ok()
            .map(|_| "direnv".to_string()),
        Style::new().fg(Color::LightBlue),
    );

    prompt.push_if(
        std::env::var("CMD_DURATION_MS")
            .ok()
            .filter(|ms| ms != "0823") // https://github.com/nushell/nushell/discussions/6402 okay????
            .and_then(|ms| ms.parse::<u64>().ok())
            .map(Duration::from_millis)
            .and_then(format_duration),
        Style::new().fg(Color::Red),
    );

    prompt.push_if(
        std::env::var("LAST_EXIT_CODE")
            .ok()
            .filter(|code| code != "0"),
        Style::new().fg(Color::Red).bold(),
    );

    prompt.print();

    Ok(())
}
