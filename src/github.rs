use std::collections::BTreeMap;
use std::env;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use reqwest::Method;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;

use crate::model::{NotificationThread, PullRequest, RepoRef};

const API_VERSION: &str = "2022-11-28";

pub trait GitHubClient {
    fn list_notifications(&self) -> Result<Vec<NotificationThread>>;
    fn get_pull_request(&self, repo: &RepoRef, number: u64) -> Result<PullRequest>;
    fn mark_thread_done(&self, thread_id: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    host: String,
    api_base: String,
    token: String,
}

impl AuthContext {
    pub fn host(&self) -> &str {
        &self.host
    }
}

#[derive(Debug)]
pub struct HttpGitHubClient {
    http: Client,
    auth: AuthContext,
}

impl HttpGitHubClient {
    pub fn new(auth: AuthContext) -> Result<Self> {
        let http = Client::builder()
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .context("failed to build the HTTP client")?;

        Ok(Self { http, auth })
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.auth.api_base.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn request(&self, method: Method, path: &str) -> reqwest::blocking::RequestBuilder {
        self.http
            .request(method, self.api_url(path))
            .bearer_auth(&self.auth.token)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
    }

    fn json<T: for<'de> Deserialize<'de>>(&self, method: Method, path: &str) -> Result<T> {
        let response = self
            .request(method, path)
            .send()
            .with_context(|| format!("request to {path} failed"))?;
        decode_json(response, path)
    }

    fn empty(&self, method: Method, path: &str) -> Result<()> {
        let response = self
            .request(method, path)
            .send()
            .with_context(|| format!("request to {path} failed"))?;
        decode_empty(response, path)
    }
}

impl GitHubClient for HttpGitHubClient {
    fn list_notifications(&self) -> Result<Vec<NotificationThread>> {
        let mut page = 1usize;
        let mut threads = Vec::new();

        loop {
            let path = format!("notifications?all=true&per_page=100&page={page}");
            let page_items: Vec<NotificationThread> = self.json(Method::GET, &path)?;
            let count = page_items.len();
            threads.extend(page_items);

            if count < 100 {
                break;
            }

            page += 1;
        }

        Ok(threads)
    }

    fn get_pull_request(&self, repo: &RepoRef, number: u64) -> Result<PullRequest> {
        let path = format!("repos/{repo}/pulls/{number}");
        self.json(Method::GET, &path)
    }

    fn mark_thread_done(&self, thread_id: &str) -> Result<()> {
        let path = format!("notifications/threads/{thread_id}");
        self.empty(Method::DELETE, &path)
    }
}

fn decode_json<T: for<'de> Deserialize<'de>>(response: Response, path: &str) -> Result<T> {
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if !status.is_success() {
        bail!("{} returned {}: {}", path, status, body.trim());
    }

    serde_json::from_str(&body)
        .with_context(|| format!("failed to decode JSON response from {path}"))
}

fn decode_empty(response: Response, path: &str) -> Result<()> {
    let status = response.status();
    let body = response.text().unwrap_or_default();

    if status.is_success() {
        return Ok(());
    }

    bail!("{} returned {}: {}", path, status, body.trim())
}

pub fn resolve_auth_context() -> Result<AuthContext> {
    let output = Command::new("gh")
        .args(["auth", "status", "--json", "hosts", "--show-token"])
        .output()
        .context("failed to run `gh auth status`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("`gh auth status` failed: {}", stderr.trim());
    }

    let decoded: AuthStatus = serde_json::from_slice(&output.stdout)
        .context("failed to decode `gh auth status` output")?;

    let preferred_host = env::var("GH_HOST").ok();
    let account = select_account(&decoded.hosts, preferred_host.as_deref())?;

    Ok(AuthContext {
        host: account.host.clone(),
        api_base: api_base_for_host(&account.host),
        token: account
            .token
            .clone()
            .ok_or_else(|| anyhow!("no auth token is available for {}", account.host))?,
    })
}

fn select_account<'a>(
    hosts: &'a BTreeMap<String, Vec<AuthHostEntry>>,
    preferred_host: Option<&str>,
) -> Result<&'a AuthHostEntry> {
    let accounts = hosts
        .values()
        .flat_map(|entries| entries.iter())
        .filter(|entry| entry.state == "success" && entry.token.is_some())
        .collect::<Vec<_>>();

    if accounts.is_empty() {
        bail!("no authenticated GitHub host is available in `gh auth status`");
    }

    if let Some(host) = preferred_host {
        return accounts
            .iter()
            .copied()
            .find(|entry| entry.host == host && entry.active)
            .or_else(|| accounts.iter().copied().find(|entry| entry.host == host))
            .ok_or_else(|| anyhow!("no authenticated account found for host `{host}`"));
    }

    if let Some(account) = accounts
        .iter()
        .copied()
        .find(|entry| entry.host == "github.com" && entry.active)
    {
        return Ok(account);
    }

    if let Some(account) = accounts.iter().copied().find(|entry| entry.active) {
        return Ok(account);
    }

    if accounts.len() == 1 {
        return Ok(accounts[0]);
    }

    bail!("multiple authenticated hosts are available; set GH_HOST to pick one")
}

fn api_base_for_host(host: &str) -> String {
    if host == "github.com" {
        "https://api.github.com".to_owned()
    } else {
        format!("https://{host}/api/v3")
    }
}

#[derive(Debug, Deserialize)]
struct AuthStatus {
    hosts: BTreeMap<String, Vec<AuthHostEntry>>,
}

#[derive(Debug, Deserialize)]
struct AuthHostEntry {
    active: bool,
    host: String,
    state: String,
    token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{AuthHostEntry, api_base_for_host, select_account};
    use std::collections::BTreeMap;

    fn account(host: &str, active: bool) -> AuthHostEntry {
        AuthHostEntry {
            active,
            host: host.to_owned(),
            state: "success".to_owned(),
            token: Some("token".to_owned()),
        }
    }

    #[test]
    fn chooses_github_dot_com_by_default() {
        let mut hosts = BTreeMap::new();
        hosts.insert("github.com".to_owned(), vec![account("github.com", true)]);
        hosts.insert(
            "ghe.example.com".to_owned(),
            vec![account("ghe.example.com", true)],
        );

        let selected = select_account(&hosts, None).expect("selected account");

        assert_eq!(selected.host, "github.com");
    }

    #[test]
    fn honors_preferred_host() {
        let mut hosts = BTreeMap::new();
        hosts.insert("github.com".to_owned(), vec![account("github.com", true)]);
        hosts.insert(
            "ghe.example.com".to_owned(),
            vec![account("ghe.example.com", true)],
        );

        let selected = select_account(&hosts, Some("ghe.example.com")).expect("selected account");

        assert_eq!(selected.host, "ghe.example.com");
    }

    #[test]
    fn builds_ghe_api_base() {
        assert_eq!(api_base_for_host("github.com"), "https://api.github.com");
        assert_eq!(
            api_base_for_host("ghe.example.com"),
            "https://ghe.example.com/api/v3"
        );
    }
}
