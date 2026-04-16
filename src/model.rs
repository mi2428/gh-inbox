use std::fmt;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NotificationThread {
    pub id: String,
    pub unread: bool,
    pub reason: String,
    pub updated_at: String,
    pub subject: NotificationSubject,
    pub repository: NotificationRepository,
    pub url: String,
    pub subscription_url: String,
}

impl NotificationThread {
    pub fn reason_is(&self, reason: &str) -> bool {
        self.reason.eq_ignore_ascii_case(reason)
    }

    pub fn pull_request_ref(&self) -> Option<PullRequestRef> {
        let subject = self.subject_ref()?;
        match subject {
            SubjectRef::PullRequest(reference) => Some(reference),
            _ => None,
        }
    }

    pub fn subject_ref(&self) -> Option<SubjectRef> {
        let url = self.subject.url.as_deref()?;
        parse_subject_url(url)
    }

    pub fn subject_summary(&self) -> String {
        let title = compact_whitespace(&self.subject.title);

        match self.subject_ref() {
            Some(SubjectRef::PullRequest(reference)) => {
                format!("PR #{}  {title}", reference.number)
            }
            Some(SubjectRef::Issue(reference)) => format!("Issue #{}  {title}", reference.number),
            None => format!("{}  {title}", self.subject.r#type),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NotificationSubject {
    pub title: String,
    pub url: Option<String>,
    pub latest_comment_url: Option<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NotificationRepository {
    pub full_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub state: String,
    pub merged_at: Option<String>,
    pub title: String,
    pub html_url: String,
    pub user: PullRequestAuthor,
}

impl PullRequest {
    pub fn is_closed_or_merged(&self) -> bool {
        self.state.eq_ignore_ascii_case("closed") || self.merged_at.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullRequestAuthor {
    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectRef {
    PullRequest(PullRequestRef),
    Issue(IssueRef),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PullRequestRef {
    pub repo: RepoRef,
    pub number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueRef {
    pub repo: RepoRef,
    pub number: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RepoRef {
    owner: String,
    name: String,
}

impl RepoRef {
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        let Some((owner, name)) = trimmed.split_once('/') else {
            bail!("repository must use OWNER/REPO form");
        };

        if owner.is_empty() || name.is_empty() {
            bail!("repository must use OWNER/REPO form");
        }

        Ok(Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
        })
    }

    pub fn matches(&self, full_name: &str) -> bool {
        self.to_string().eq_ignore_ascii_case(full_name)
    }
}

impl fmt::Display for RepoRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.name)
    }
}

pub fn parse_subject_url(input: &str) -> Option<SubjectRef> {
    let url = Url::parse(input).ok()?;
    let segments = url.path_segments()?.collect::<Vec<_>>();
    let repos_index = segments.iter().position(|segment| *segment == "repos")?;
    let owner = segments.get(repos_index + 1)?.to_string();
    let name = segments.get(repos_index + 2)?.to_string();
    let repo = RepoRef { owner, name };
    let kind = *segments.get(repos_index + 3)?;
    let number = segments.get(repos_index + 4)?.parse::<u64>().ok()?;

    match kind {
        "pulls" => Some(SubjectRef::PullRequest(PullRequestRef { repo, number })),
        "issues" => Some(SubjectRef::Issue(IssueRef { repo, number })),
        _ => None,
    }
}

fn compact_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        NotificationRepository, NotificationSubject, NotificationThread, RepoRef, SubjectRef,
        parse_subject_url,
    };

    #[test]
    fn parses_github_dot_com_pr_subject_urls() {
        let parsed = parse_subject_url("https://api.github.com/repos/cli/cli/pulls/123")
            .expect("parsed subject");

        match parsed {
            SubjectRef::PullRequest(reference) => {
                assert_eq!(reference.repo.to_string(), "cli/cli");
                assert_eq!(reference.number, 123);
            }
            other => panic!("expected a pull request reference, got {other:?}"),
        }
    }

    #[test]
    fn parses_ghe_pr_subject_urls() {
        let parsed = parse_subject_url("https://ghe.example.com/api/v3/repos/acme/widgets/pulls/9")
            .expect("parsed subject");

        match parsed {
            SubjectRef::PullRequest(reference) => {
                assert_eq!(reference.repo.to_string(), "acme/widgets");
                assert_eq!(reference.number, 9);
            }
            other => panic!("expected a pull request reference, got {other:?}"),
        }
    }

    #[test]
    fn formats_pull_request_subject_summary() {
        let thread = NotificationThread {
            id: "1".to_owned(),
            unread: true,
            reason: "review_requested".to_owned(),
            updated_at: "2026-04-16T00:00:00Z".to_owned(),
            subject: NotificationSubject {
                title: "  Fix   whitespace\nin output ".to_owned(),
                url: Some("https://api.github.com/repos/cli/cli/pulls/123".to_owned()),
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

        assert_eq!(
            thread.subject_summary(),
            "PR #123  Fix whitespace in output"
        );
    }

    #[test]
    fn parses_repo_ref() {
        let repo = RepoRef::parse("cli/cli").expect("repo ref");
        assert_eq!(repo.to_string(), "cli/cli");
    }
}
