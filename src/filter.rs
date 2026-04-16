use anyhow::Result;

use crate::cli::SweepArgs;
use crate::model::{NotificationThread, PullRequest, RepoRef};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SweepFilters {
    pub read: bool,
    pub closed: bool,
    pub repo: Option<RepoRef>,
    pub user: Option<String>,
    pub team_mentioned: bool,
    pub no_mentioned: bool,
}

impl SweepFilters {
    pub fn needs_pull_request_metadata(&self) -> bool {
        self.closed || self.user.is_some()
    }

    pub fn matches(&self, thread: &NotificationThread, pr: Option<&PullRequest>) -> bool {
        if self.read && thread.unread {
            return false;
        }

        if let Some(repo) = &self.repo
            && !repo.matches(&thread.repository.full_name)
        {
            return false;
        }

        if self.team_mentioned && !thread.reason_is("team_mention") {
            return false;
        }

        if self.no_mentioned && thread.reason_is("mention") {
            return false;
        }

        if self.closed || self.user.is_some() {
            let Some(pr) = pr else {
                return false;
            };

            if self.closed && !pr.is_closed_or_merged() {
                return false;
            }

            if let Some(user) = &self.user
                && !pr.user.login.eq_ignore_ascii_case(user)
            {
                return false;
            }
        }

        true
    }
}

impl TryFrom<SweepArgs> for SweepFilters {
    type Error = anyhow::Error;

    fn try_from(value: SweepArgs) -> Result<Self> {
        Ok(Self {
            read: value.read,
            closed: value.closed,
            repo: value.repo.map(|repo| RepoRef::parse(&repo)).transpose()?,
            user: value.user,
            team_mentioned: value.team_mentioned,
            no_mentioned: value.no_mentioned,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SweepFilters;
    use crate::cli::SweepArgs;
    use crate::model::{
        NotificationRepository, NotificationSubject, NotificationThread, PullRequest,
        PullRequestAuthor,
    };

    fn thread(
        reason: &str,
        repo: &str,
        subject_type: &str,
        subject_url: Option<&str>,
    ) -> NotificationThread {
        NotificationThread {
            id: "1".to_owned(),
            unread: true,
            reason: reason.to_owned(),
            updated_at: "2026-04-16T00:00:00Z".to_owned(),
            subject: NotificationSubject {
                title: "Test subject".to_owned(),
                url: subject_url.map(ToOwned::to_owned),
                latest_comment_url: None,
                r#type: subject_type.to_owned(),
            },
            repository: NotificationRepository {
                full_name: repo.to_owned(),
            },
            url: "https://api.github.com/notifications/threads/1".to_owned(),
            subscription_url: "https://api.github.com/notifications/threads/1/subscription"
                .to_owned(),
        }
    }

    fn pr(state: &str, author: &str, merged: bool) -> PullRequest {
        PullRequest {
            number: 42,
            state: state.to_owned(),
            merged_at: merged.then(|| "2026-04-16T00:00:00Z".to_owned()),
            title: "Test pull request".to_owned(),
            html_url: "https://github.com/cli/cli/pull/42".to_owned(),
            user: PullRequestAuthor {
                login: author.to_owned(),
            },
        }
    }

    fn read_thread(
        reason: &str,
        repo: &str,
        subject_type: &str,
        subject_url: Option<&str>,
    ) -> NotificationThread {
        let mut thread = thread(reason, repo, subject_type, subject_url);
        thread.unread = false;
        thread
    }

    #[test]
    fn matches_without_filters() {
        let filters = SweepFilters::default();
        let thread = thread(
            "comment",
            "cli/cli",
            "Issue",
            Some("https://api.github.com/repos/cli/cli/issues/1"),
        );

        assert!(filters.matches(&thread, None));
    }

    #[test]
    fn matches_repo_filter_case_insensitively() {
        let filters = SweepFilters {
            repo: Some(crate::model::RepoRef::new_unchecked("CLI/Cli".to_owned())),
            ..SweepFilters::default()
        };
        let thread = thread(
            "comment",
            "cli/cli",
            "Issue",
            Some("https://api.github.com/repos/cli/cli/issues/1"),
        );

        assert!(filters.matches(&thread, None));
    }

    #[test]
    fn read_filter_only_matches_read_notifications() {
        let filters = SweepFilters {
            read: true,
            ..SweepFilters::default()
        };

        assert!(filters.matches(
            &read_thread(
                "comment",
                "cli/cli",
                "Issue",
                Some("https://api.github.com/repos/cli/cli/issues/1")
            ),
            None
        ));
        assert!(!filters.matches(
            &thread(
                "comment",
                "cli/cli",
                "Issue",
                Some("https://api.github.com/repos/cli/cli/issues/1")
            ),
            None
        ));
    }

    #[test]
    fn rejects_non_matching_repo() {
        let filters = SweepFilters {
            repo: Some(crate::model::RepoRef::new_unchecked("cli/other".to_owned())),
            ..SweepFilters::default()
        };
        let thread = thread(
            "comment",
            "cli/cli",
            "Issue",
            Some("https://api.github.com/repos/cli/cli/issues/1"),
        );

        assert!(!filters.matches(&thread, None));
    }

    #[test]
    fn matches_team_mention_only() {
        let filters = SweepFilters {
            team_mentioned: true,
            ..SweepFilters::default()
        };

        assert!(filters.matches(
            &thread(
                "team_mention",
                "cli/cli",
                "Issue",
                Some("https://api.github.com/repos/cli/cli/issues/1")
            ),
            None
        ));
        assert!(!filters.matches(
            &thread(
                "mention",
                "cli/cli",
                "Issue",
                Some("https://api.github.com/repos/cli/cli/issues/1")
            ),
            None
        ));
    }

    #[test]
    fn no_mentioned_excludes_direct_mentions() {
        let filters = SweepFilters {
            no_mentioned: true,
            ..SweepFilters::default()
        };

        assert!(filters.matches(
            &thread(
                "team_mention",
                "cli/cli",
                "Issue",
                Some("https://api.github.com/repos/cli/cli/issues/1")
            ),
            None
        ));
        assert!(!filters.matches(
            &thread(
                "mention",
                "cli/cli",
                "Issue",
                Some("https://api.github.com/repos/cli/cli/issues/1")
            ),
            None
        ));
    }

    #[test]
    fn closed_requires_pull_request_metadata() {
        let filters = SweepFilters {
            closed: true,
            ..SweepFilters::default()
        };
        let thread = thread(
            "review_requested",
            "cli/cli",
            "PullRequest",
            Some("https://api.github.com/repos/cli/cli/pulls/42"),
        );

        assert!(filters.matches(&thread, Some(&pr("closed", "monalisa", false))));
        assert!(!filters.matches(&thread, Some(&pr("open", "monalisa", false))));
        assert!(!filters.matches(&thread, None));
    }

    #[test]
    fn user_filter_requires_matching_pr_author() {
        let filters = SweepFilters {
            user: Some("MonaLisa".to_owned()),
            ..SweepFilters::default()
        };
        let thread = thread(
            "review_requested",
            "cli/cli",
            "PullRequest",
            Some("https://api.github.com/repos/cli/cli/pulls/42"),
        );

        assert!(filters.matches(&thread, Some(&pr("open", "monalisa", false))));
        assert!(!filters.matches(&thread, Some(&pr("open", "hubot", false))));
    }

    #[test]
    fn combined_filters_are_anded() {
        let filters = SweepFilters {
            read: true,
            closed: true,
            repo: Some(crate::model::RepoRef::new_unchecked("cli/cli".to_owned())),
            user: Some("monalisa".to_owned()),
            team_mentioned: true,
            no_mentioned: true,
        };
        let thread = read_thread(
            "team_mention",
            "cli/cli",
            "PullRequest",
            Some("https://api.github.com/repos/cli/cli/pulls/42"),
        );

        assert!(filters.matches(&thread, Some(&pr("closed", "monalisa", true))));
        assert!(!filters.matches(&thread, Some(&pr("open", "monalisa", false))));
    }

    #[test]
    fn rejects_invalid_repo_filters() {
        let args = SweepArgs {
            read: false,
            closed: false,
            repo: Some("invalid".to_owned()),
            user: None,
            team_mentioned: false,
            no_mentioned: false,
        };

        assert!(SweepFilters::try_from(args).is_err());
    }
}
