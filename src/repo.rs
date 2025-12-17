use std::path::{
    Path,
    PathBuf,
};

use jj_lib::{
    id_prefix::IdPrefixIndex,
    repo::{
        Repo,
        StoreFactories,
    },
    settings::UserSettings,
    workspace::{
        Workspace,
        default_working_copy_factories,
    },
};
use nu_ansi_term::{
    Color,
    Style,
};

use crate::prompt::{
    Part,
    Segment,
};

#[derive(Debug)]
pub struct GitContext {
    branch: String,
    root:   PathBuf,
    dirty:  bool,
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

#[derive(Debug, Clone)]
struct ChangeInfo {
    hex:        String,
    prefix_len: usize,
}

#[derive(Debug)]
pub struct JujutsuContext {
    change_info: ChangeInfo,
    root:        PathBuf,
    dirty:       bool,
}

impl JujutsuContext {
    fn discover_jj(cwd: &Path) -> Option<Self> {
        let workspace_root =
            cwd.ancestors().find(|path| path.join(".jj").is_dir())?;

        let user_settings = UserSettings::from_config(
            jj_lib::config::StackedConfig::with_defaults(),
        )
        .expect("Can't load settings"); // TODO: if necessary, load actual settings

        // TODO: Proper error handling
        let store_factories = StoreFactories::default();
        let wc_factories = default_working_copy_factories();
        let workspace = Workspace::load(
            &user_settings,
            workspace_root,
            &store_factories,
            &wc_factories,
        )
        .expect("Could not load jj workspace");

        let repo = workspace
            .repo_loader()
            .load_at_head()
            .expect("Could not load jj repo head");

        let workspace_name = workspace.workspace_name();
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace_name)
            .expect("Could not get jj commit ID");

        let commit = repo
            .store()
            .get_commit(wc_commit_id)
            .expect("Could not resolve jj commit");

        // FIXME: prefixes are based on ENTIRE repo index, not just "current"
        // work tree. ie, revset 'trunk()..@'. Fixing this with only
        // jj-lib sucks afaict. I have concluded that this is super
        // annoying to fix, so I'm not going to do that right now.
        let id_idx = IdPrefixIndex::empty();

        let change_prefix_len = id_idx
            .shortest_change_prefix_len(repo.as_ref(), commit.change_id())
            .expect("Could not calculate shortest prefix for change");

        let change_info = ChangeInfo {
            hex:        commit.change_id().reverse_hex(),
            prefix_len: change_prefix_len,
        };

        // TODO: This works, but only updates after manual `jj <anything>`
        // This is arguably okay! But we could rework this and use caching
        // and then manually trigger a reload on invalidation. (op_id)
        let discardable = commit
            .is_discardable(&*repo)
            .expect("Could not read is_discarable");

        Some(Self {
            change_info,
            root: workspace_root.to_path_buf(),
            dirty: !discardable,
        })
    }
}

#[derive(Debug)]
pub enum RepoContext {
    Git(GitContext),
    Jujutsu(JujutsuContext),
}

// TODO:
// Ideally, should probably define these colors elsewhere.
// I am going to have to break apart this API soon.
// For now, get it working, get it running.
impl RepoContext {
    pub fn discover(cwd: &Path) -> Option<Self> {
        JujutsuContext::discover_jj(cwd)
            .map(Self::Jujutsu)
            .or_else(|| GitContext::discover(cwd).map(Self::Git))
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        match self {
            Self::Git(ctx) => &ctx.root,
            Self::Jujutsu(ctx) => &ctx.root,
        }
    }

    #[must_use]
    pub const fn dirty(&self) -> bool {
        match self {
            Self::Git(ctx) => ctx.dirty,
            Self::Jujutsu(ctx) => ctx.dirty,
        }
    }

    #[must_use]
    pub fn reference(&self) -> String {
        match self {
            Self::Git(ctx) => {
                Segment {
                    text:  ctx.branch.clone(),
                    style: Style::new().fg(Color::Green),
                }
                .to_string()
            }, // FIX Don't set colors here probably
            Self::Jujutsu(ctx) => {
                // FIX This is... okay.
                const MAX_LEN: usize = 8;

                let hex = ctx.change_info.hex.clone();
                let prefix_len = ctx.change_info.prefix_len;
                let prefix = &hex[..prefix_len];
                let suffix = &hex[prefix_len..MAX_LEN];

                Part(vec![
                    Segment {
                        text:  prefix.to_string(),
                        style: Style::new().fg(Color::Yellow).bold(),
                    },
                    Segment {
                        text:  suffix.to_string(),
                        style: Style::new().fg(Color::DarkGray).bold(),
                    },
                ])
                .to_string()
            },
        }
    }
}
