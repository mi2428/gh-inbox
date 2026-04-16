mod cli;
mod filter;
mod github;
mod model;
mod output;
mod progress;

use std::io::{self, Write};

use anyhow::{Result, anyhow};
use clap::Parser;

use crate::cli::{Cli, Commands, SweepArgs};
use crate::filter::SweepFilters;
use crate::github::{GitHubClient, HttpGitHubClient, resolve_auth_context};
use crate::model::{NotificationThread, PullRequest};
use crate::output::{ListRow, write_list};
use crate::progress::SweepProgress;

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::List => run_list(),
        Commands::Sweep(args) => run_sweep(args),
    }
}

fn run_list() -> Result<()> {
    let auth = resolve_auth_context()?;
    let client = HttpGitHubClient::new(auth)?;
    let notifications = client.list_notifications()?;

    let rows = notifications
        .iter()
        .map(|thread| ListRow {
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
    let auth = resolve_auth_context()?;
    let filters = SweepFilters::build(args, auth.login().to_owned())?;
    let client = HttpGitHubClient::new(auth)?;
    let notifications = client.list_notifications()?;
    let filter_progress = SweepProgress::new(notifications.len(), "Filtering");

    let mut candidates = Vec::new();
    let mut metadata_failures = Vec::new();

    for thread in notifications {
        let pr = match pr_metadata_for_thread(&client, &filters, &thread) {
            Ok(pr) => pr,
            Err(error) => {
                metadata_failures.push((thread, error.to_string()));
                filter_progress.inc(1);
                continue;
            }
        };

        if filters.matches(&thread, pr.as_ref()) {
            candidates.push(thread);
        }

        filter_progress.inc(1);
    }
    filter_progress.finish();

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
    let mark_progress = SweepProgress::new(candidates.len(), "Marking done");

    for thread in &candidates {
        match client.mark_thread_done(&thread.id) {
            Ok(()) => {
                done += 1;
            }
            Err(error) => failures.push((thread.clone(), error.to_string())),
        }
        mark_progress.inc(1);
    }
    mark_progress.finish();

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
