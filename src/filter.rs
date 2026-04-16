use anyhow::Result;

use crate::cli::SweepArgs;
use crate::model::{NotificationThread, PullRequest, RepoRef};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SweepFilters {
    pub closed: bool,
    pub repo: Option<RepoRef>,
    pub user: Option<String>,
    pub team_mentioned: bool,
    pub no_mentioned: bool,
    pub include_authored: bool,
    pub viewer_login: Option<String>,
}

impl SweepFilters {
    pub fn build(args: SweepArgs, viewer_login: String) -> Result<Self> {
        Ok(Self {
            closed: args.closed,
            repo: args.repo.map(|repo| RepoRef::parse(&repo)).transpose()?,
            user: args.user,
            team_mentioned: args.team_mentioned,
            no_mentioned: args.no_mentioned,
            include_authored: args.include_authored,
            viewer_login: Some(viewer_login),
        })
    }

    pub fn needs_pull_request_metadata(&self) -> bool {
        self.closed
            || self.user.is_some()
            || (!self.include_authored && self.viewer_login.is_some())
    }

    pub fn matches(&self, thread: &NotificationThread, pr: Option<&PullRequest>) -> bool {
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

        let needs_pr = self.closed
            || self.user.is_some()
            || (!self.include_authored
                && self.viewer_login.is_some()
                && thread.pull_request_ref().is_some());

        if !needs_pr {
            return true;
        }

        let Some(pr) = pr else {
            return false;
        };

        // Protect the authenticated user's own pull requests unless explicitly overridden.
        if !self.include_authored
            && let Some(viewer_login) = &self.viewer_login
            && pr.user.login.eq_ignore_ascii_case(viewer_login)
        {
            return false;
        }

        if self.closed && !pr.is_closed_or_merged() {
            return false;
        }

        if let Some(user) = &self.user
            && !pr.user.login.eq_ignore_ascii_case(user)
        {
            return false;
        }

        true
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

    fn sweep_args() -> SweepArgs {
        SweepArgs {
            closed: false,
            repo: None,
            user: None,
            team_mentioned: false,
            no_mentioned: false,
            include_authored: false,
        }
    }

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

    #[test]
    fn builds_filters_from_args() {
        let filters = SweepFilters::build(
            SweepArgs {
                closed: true,
                repo: Some("cli/cli".to_owned()),
                user: Some("monalisa".to_owned()),
                team_mentioned: true,
                no_mentioned: true,
                include_authored: true,
            },
            "hubot".to_owned(),
        )
        .expect("built filters");

        assert!(filters.closed);
        assert_eq!(filters.repo.expect("repo").to_string(), "cli/cli");
        assert_eq!(filters.user.expect("user"), "monalisa");
        assert!(filters.team_mentioned);
        assert!(filters.no_mentioned);
        assert!(filters.include_authored);
        assert_eq!(filters.viewer_login.as_deref(), Some("hubot"));
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
            repo: Some(crate::model::RepoRef::parse("CLI/Cli").expect("repo")),
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
    fn rejects_non_matching_repo() {
        let filters = SweepFilters {
            repo: Some(crate::model::RepoRef::parse("cli/other").expect("repo")),
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
    fn protects_self_authored_pull_requests_by_default() {
        let filters = SweepFilters::build(sweep_args(), "monalisa".to_owned()).expect("filters");
        let thread = thread(
            "review_requested",
            "cli/cli",
            "PullRequest",
            Some("https://api.github.com/repos/cli/cli/pulls/42"),
        );

        assert!(!filters.matches(&thread, Some(&pr("open", "monalisa", false))));
        assert!(filters.matches(&thread, Some(&pr("open", "hubot", false))));
    }

    #[test]
    fn include_authored_disables_self_authored_protection() {
        let filters = SweepFilters::build(
            SweepArgs {
                include_authored: true,
                ..sweep_args()
            },
            "monalisa".to_owned(),
        )
        .expect("filters");
        let thread = thread(
            "review_requested",
            "cli/cli",
            "PullRequest",
            Some("https://api.github.com/repos/cli/cli/pulls/42"),
        );

        assert!(filters.matches(&thread, Some(&pr("open", "monalisa", false))));
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
    fn self_authored_protection_requires_pull_request_metadata() {
        let filters = SweepFilters::build(sweep_args(), "monalisa".to_owned()).expect("filters");
        let thread = thread(
            "review_requested",
            "cli/cli",
            "PullRequest",
            Some("https://api.github.com/repos/cli/cli/pulls/42"),
        );

        assert!(filters.needs_pull_request_metadata());
        assert!(!filters.matches(&thread, None));
    }

    #[test]
    fn user_filter_requires_matching_pr_author() {
        let filters = SweepFilters {
            user: Some("MonaLisa".to_owned()),
            include_authored: true,
            viewer_login: Some("hubot".to_owned()),
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
            closed: true,
            repo: Some(crate::model::RepoRef::parse("cli/cli").expect("repo")),
            user: Some("monalisa".to_owned()),
            team_mentioned: true,
            no_mentioned: true,
            include_authored: true,
            viewer_login: Some("hubot".to_owned()),
        };
        let thread = thread(
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
            repo: Some("invalid".to_owned()),
            ..sweep_args()
        };

        assert!(SweepFilters::build(args, "monalisa".to_owned()).is_err());
    }
}
