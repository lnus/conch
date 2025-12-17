use std::time::Duration;

use anyhow::Result;
use conch::{
    prompt::Prompt,
    repo::RepoContext,
    util::{
        format_duration,
        format_path,
        format_repo,
    },
};
use nu_ansi_term::{
    Color,
    Style,
};

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
        (false, true) => {
            Prompt::new()
                .with_separator(" • ")
                .with_prefix("╭─ ")
                .with_suffix("\n│")
        },
        (false, false) => Prompt::new().with_separator(" ∵ "),
    }
    .with_style(Style::new().fg(Color::Yellow));

    prompt.push(
        format_path(&cwd, repo.as_ref()),
        Style::new().fg(Color::Cyan).bold(),
    );

    // Forcing a style here is... not ideal
    // TODO Consider push_if definition
    prompt.push_if(repo.as_ref().map(format_repo), Style::default());

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

    // https://github.com/nushell/nushell/discussions/6402 okay????
    prompt.push_if(
        std::env::var("CMD_DURATION_MS")
            .ok()
            .filter(|ms| ms != "0823")
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
