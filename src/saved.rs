use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::model::{NotificationThread, PullRequestRef, RepoRef};

const REGISTRY_FILENAME: &str = "saved-pull-requests.json";

#[derive(Debug)]
pub struct SavedRegistry {
    path: PathBuf,
    entries: BTreeSet<SavedPullRequest>,
}

impl SavedRegistry {
    pub fn load() -> Result<Self> {
        let path = registry_path()?;
        Self::load_from_path(path)
    }

    pub fn load_from_path(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                path,
                entries: BTreeSet::new(),
            });
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let file: SavedRegistryFile = serde_json::from_str(&raw)
            .with_context(|| format!("failed to decode {}", path.display()))?;

        Ok(Self {
            path,
            entries: file
                .entries
                .into_iter()
                .map(SavedPullRequest::from)
                .collect(),
        })
    }

    pub fn contains_notification(&self, host: &str, thread: &NotificationThread) -> bool {
        let Some(PullRequestRef { repo, number }) = thread.pull_request_ref() else {
            return false;
        };

        let key = SavedPullRequest {
            host: host.to_owned(),
            repo,
            number,
            title: None,
            saved_at: None,
        };

        self.entries.contains(&key)
    }

    pub fn save_pull_request(
        &mut self,
        host: &str,
        repo: &RepoRef,
        number: u64,
        title: &str,
    ) -> Result<bool> {
        let saved_at = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let entry = SavedPullRequest {
            host: host.to_owned(),
            repo: repo.clone(),
            number,
            title: Some(title.to_owned()),
            saved_at: Some(saved_at),
        };
        let added = self.entries.insert(entry);
        self.persist()?;
        Ok(added)
    }

    fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let file = SavedRegistryFile {
            version: 1,
            entries: self
                .entries
                .iter()
                .cloned()
                .map(SavedPullRequestFile::from)
                .collect(),
        };
        let serialized = serde_json::to_string_pretty(&file)?;
        fs::write(&self.path, serialized)
            .with_context(|| format!("failed to write {}", self.path.display()))?;
        Ok(())
    }
}

fn registry_path() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("GH_INBOX_CONFIG_DIR") {
        return Ok(Path::new(&dir).join(REGISTRY_FILENAME));
    }

    let config_dir = dirs::config_dir().context("failed to locate the user config directory")?;
    Ok(config_dir.join("gh-inbox").join(REGISTRY_FILENAME))
}

#[derive(Debug, Clone)]
struct SavedPullRequest {
    host: String,
    repo: RepoRef,
    number: u64,
    title: Option<String>,
    saved_at: Option<String>,
}

impl PartialEq for SavedPullRequest {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.repo == other.repo && self.number == other.number
    }
}

impl Eq for SavedPullRequest {}

impl PartialOrd for SavedPullRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SavedPullRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        (&self.host, &self.repo, self.number).cmp(&(&other.host, &other.repo, other.number))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedRegistryFile {
    version: u32,
    entries: Vec<SavedPullRequestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedPullRequestFile {
    host: String,
    repo: String,
    number: u64,
    title: Option<String>,
    saved_at: Option<String>,
}

impl From<SavedPullRequestFile> for SavedPullRequest {
    fn from(value: SavedPullRequestFile) -> Self {
        Self {
            host: value.host,
            repo: RepoRef::new_unchecked(value.repo),
            number: value.number,
            title: value.title,
            saved_at: value.saved_at,
        }
    }
}

impl From<SavedPullRequest> for SavedPullRequestFile {
    fn from(value: SavedPullRequest) -> Self {
        Self {
            host: value.host,
            repo: value.repo.to_string(),
            number: value.number,
            title: value.title,
            saved_at: value.saved_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempfile::tempdir;

    use super::SavedRegistry;
    use crate::model::{NotificationRepository, NotificationSubject, NotificationThread, RepoRef};

    #[test]
    fn persists_saved_pull_requests() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("saved-pull-requests.json");
        let repo = RepoRef::parse("cli/cli")?;

        let mut registry = SavedRegistry::load_from_path(path.clone())?;
        assert!(registry.save_pull_request("github.com", &repo, 42, "Example PR")?);
        assert!(!registry.save_pull_request("github.com", &repo, 42, "Example PR")?);

        let loaded = SavedRegistry::load_from_path(path)?;
        let thread = NotificationThread {
            id: "1".to_owned(),
            unread: true,
            reason: "review_requested".to_owned(),
            updated_at: "2026-04-16T00:00:00Z".to_owned(),
            subject: NotificationSubject {
                title: "Example PR".to_owned(),
                url: Some("https://api.github.com/repos/cli/cli/pulls/42".to_owned()),
                latest_comment_url: None,
                r#type: "PullRequest".to_owned(),
            },
            repository: NotificationRepository {
                full_name: "cli/cli".to_owned(),
            },
            url: "https://api.github.com/notifications/threads/1".to_owned(),
            subscription_url: "https://api.github.com/notifications/threads/1/subscription"
                .to_owned(),
        };

        assert!(loaded.contains_notification("github.com", &thread));
        assert!(!loaded.contains_notification("ghe.example.com", &thread));
        Ok(())
    }
}
