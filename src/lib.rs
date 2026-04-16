mod cli;
mod filter;
mod github;
mod model;
mod output;
mod saved;

use std::io::{self, Write};

use anyhow::{Result, anyhow};
use clap::Parser;

use crate::cli::{Cli, Commands, SaveArgs, SweepArgs};
use crate::filter::SweepFilters;
use crate::github::{GitHubClient, HttpGitHubClient, resolve_auth_context};
use crate::model::{NotificationThread, PullRequest, RepoRef};
use crate::output::{ListRow, write_list};
use crate::saved::SavedRegistry;

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => run_list(),
        Commands::Sweep(args) => run_sweep(args),
        Commands::Save(args) => run_save(args),
    }
}

fn run_list() -> Result<()> {
    let auth = resolve_auth_context()?;
    let client = HttpGitHubClient::new(auth.clone())?;
    let saved = SavedRegistry::load()?;
    let notifications = client.list_notifications()?;

    let rows = notifications
        .iter()
        .map(|thread| ListRow {
            saved: saved.contains_notification(auth.host(), thread),
            status: if thread.unread { "unread" } else { "read" },
            reason: thread.reason.clone(),
            repository: thread.repository.full_name.clone(),
            subject: thread.subject_summary(),
        })
        .collect::<Vec<_>>();

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write_list(&mut handle, &rows)?;
    handle.flush()?;
    Ok(())
}

fn run_sweep(args: SweepArgs) -> Result<()> {
    let filters = SweepFilters::try_from(args)?;
    let auth = resolve_auth_context()?;
    let client = HttpGitHubClient::new(auth.clone())?;
    let saved = SavedRegistry::load()?;
    let notifications = client.list_notifications()?;

    let mut candidates = Vec::new();
    let mut metadata_failures = Vec::new();

    for thread in notifications {
        let is_saved = saved.contains_notification(auth.host(), &thread);
        if is_saved {
            continue;
        }

        let pr = match pr_metadata_for_thread(&client, &filters, &thread) {
            Ok(pr) => pr,
            Err(error) => {
                metadata_failures.push((thread, error.to_string()));
                continue;
            }
        };

        if filters.matches(&thread, pr.as_ref()) {
            candidates.push(thread);
        }
    }

    if !metadata_failures.is_empty() {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        for (thread, error) in &metadata_failures {
            writeln!(
                handle,
                "warning: skipped {} in {}: {}",
                thread.subject_summary(),
                thread.repository.full_name,
                error
            )?;
        }
        handle.flush()?;
    }

    if candidates.is_empty() {
        println!("No notifications matched the current filter.");
        return Ok(());
    }

    let mut failures = Vec::new();
    let mut done = 0usize;

    for thread in &candidates {
        match client.mark_thread_done(&thread.id) {
            Ok(()) => {
                done += 1;
            }
            Err(error) => failures.push((thread.clone(), error.to_string())),
        }
    }

    println!("Marked {done} notification(s) as done.");

    if failures.is_empty() {
        return Ok(());
    }

    let stderr = io::stderr();
    let mut handle = stderr.lock();
    for (thread, error) in &failures {
        writeln!(
            handle,
            "error: failed to mark {} in {} as done: {}",
            thread.subject_summary(),
            thread.repository.full_name,
            error
        )?;
    }
    handle.flush()?;

    Err(anyhow!(
        "failed to mark {} notification(s) as done",
        failures.len()
    ))
}

fn pr_metadata_for_thread(
    client: &impl GitHubClient,
    filters: &SweepFilters,
    thread: &NotificationThread,
) -> Result<Option<PullRequest>> {
    if !filters.needs_pull_request_metadata() {
        return Ok(None);
    }

    let Some(reference) = thread.pull_request_ref() else {
        return Ok(None);
    };

    let pr = client.get_pull_request(&reference.repo, reference.number)?;
    Ok(Some(pr))
}

fn run_save(args: SaveArgs) -> Result<()> {
    let auth = resolve_auth_context()?;
    let client = HttpGitHubClient::new(auth.clone())?;
    let repo = RepoRef::parse(&args.repo)?;
    let pr = client.get_pull_request(&repo, args.pr)?;

    let mut saved = SavedRegistry::load()?;
    let added = saved.save_pull_request(auth.host(), &repo, pr.number, &pr.title)?;

    if added {
        println!(
            "Saved PR #{} in {} locally. Future sweeps will skip it.",
            pr.number, repo
        );
    } else {
        println!("PR #{} in {} is already saved locally.", pr.number, repo);
    }

    Ok(())
}
