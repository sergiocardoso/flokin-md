use std::time::Duration;

use flokin_core::ReleaseInfo;

/// GitHub Releases API endpoint for FlokinMD. No authentication is required for public,
/// unauthenticated read access, and this stays well under GitHub's anonymous rate limit
/// for a check that runs at most once every 24 hours plus occasional manual checks.
const RELEASES_API_URL: &str = "https://api.github.com/repos/sergiocardoso/flokin-md/releases";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);

fn user_agent() -> String {
    format!(
        "FlokinMD/{} (+https://github.com/sergiocardoso/flokin-md)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Fetches the release list from GitHub and parses it into release metadata.
/// Runs synchronously; callers must run it off the UI thread (e.g. inside `Task::perform`).
/// Never panics: network failures and malformed responses both surface as an `Err(String)`.
pub fn fetch_releases() -> Result<Vec<ReleaseInfo>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build()
        .new_agent();

    let body = agent
        .get(RELEASES_API_URL)
        .header("User-Agent", user_agent())
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|error| format!("Não foi possível verificar atualizações: {error}"))?
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Não foi possível ler a resposta do GitHub: {error}"))?;

    flokin_core::parse_github_releases(&body)
}

#[cfg(test)]
mod tests {
    use super::user_agent;

    #[test]
    fn user_agent_identifies_flokinmd_with_its_version() {
        let agent = user_agent();
        assert!(agent.starts_with("FlokinMD/"));
        assert!(agent.contains(env!("CARGO_PKG_VERSION")));
    }
}
