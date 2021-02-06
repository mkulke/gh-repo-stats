#[cfg(test)]
use mockito::server_url;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::error::Error;
use structopt::StructOpt;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

#[derive(Debug, StructOpt)]
#[structopt(about = "Retrieve GitHub repo stats")]
struct Opt {
    #[structopt(short, long, env = "GITHUB_TOKEN", hide_env_values = true)]
    github_token: String,

    #[structopt(short, long, help = "Consider archived repositories")]
    archived: bool,

    #[structopt(short, long, help = "Organization", default_value = "microsoft")]
    org: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Repo {
    name: String,
    topics: Vec<String>,
    archived: bool,
    language: Option<String>,
    size: u32,
}

#[derive(Serialize, Debug)]
struct RepoCsvRow {
    name: String,
    topics: String,
    language: Option<String>,
    size: u32,
}

fn find_next_page(headers: &HeaderMap) -> Option<u32> {
    let link_header = headers.get("link")?.to_str().ok()?;
    let links = parse_link_header::parse(link_header).ok()?;
    let link = links.get(&Some("next".into()))?;
    let page = link.queries.get("page")?;
    page.parse::<u32>().ok()
}

fn retrieve_repos<'a>(
    github_token: &'a str,
    org: &'a str,
    archived: bool,
) -> Box<dyn Iterator<Item = Repo> + 'a> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap();

    let mut next_page = Some(1);
    let repos = std::iter::from_fn(move || {
        let page = next_page?;

        #[cfg(not(test))]
        let base_url = "https://api.github.com";

        #[cfg(test)]
        let base_url = &server_url();

        let request_url = format!(
            "{base_url}/orgs/{org}/repos?per_page={per_page}&page={page}",
            org = org,
            base_url = base_url,
            per_page = 25,
            page = page
        );

        let response = client
            .get(&request_url)
            .bearer_auth(&github_token)
            .header("Accept", "application/vnd.github.mercy-preview+json")
            .send()
            .unwrap();

        if response.status() != 200 {
            panic!("request failed: {:?}", response.text());
        }

        next_page = find_next_page(&response.headers());
        let repos: Vec<Repo> = response.json().unwrap();
        Some(repos)
    })
    .flatten()
    .filter(move |r| !r.archived || archived);

    Box::new(repos)
}

fn main() -> Result<(), Box<dyn Error>> {
    let Opt {
        github_token,
        archived,
        org,
    } = Opt::from_args();

    let mut wtr = csv::Writer::from_writer(std::io::stdout());

    for repo in retrieve_repos(&github_token, &org, archived) {
        let row = RepoCsvRow {
            name: repo.name,
            topics: repo.topics.join(" "),
            language: repo.language,
            size: repo.size,
        };
        wtr.serialize(&row)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;
    use serde_json::json;

    #[test]
    fn retrieve() {
        let github_mock = mock("GET", "/orgs/some-org/repos?per_page=25&page=1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!([{"name": "some-name", "topics": [], "archived": false, "size": 1}])
                    .to_string(),
            )
            .create();

        let mut repos = retrieve_repos("some-token", "some-org", true);
        let repo = repos.next().unwrap();

        github_mock.assert();
        assert_eq!(repo.name, "some-name");
    }

    #[test]
    fn retrieve_paginated() {
        let link = r#"<https://api.github.com/orgs/some-org/repos?per_page=25&page=2>; rel="next""#;
        let page_1 = mock("GET", "/orgs/some-org/repos?per_page=25&page=1")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_header("link", link)
            .with_body(
                json!([{"name": "some-name", "topics": [], "archived": false, "size": 1}])
                    .to_string(),
            )
            .create();

        let page_2 = mock("GET", "/orgs/some-org/repos?per_page=25&page=2")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!([{"name": "other-name", "topics": [], "archived": false, "size": 1}])
                    .to_string(),
            )
            .create();

        let mut repos = retrieve_repos("some-token", "some-org", true);
        let _ = repos.next().unwrap();
        page_1.assert();
        let repo = repos.next().unwrap();
        page_2.assert();
        assert_eq!(repo.name, "other-name");
        let none = repos.next();
        assert!(none.is_none());
    }
}
