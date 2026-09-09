use semver::Version;
use serde::Deserialize;

/// Update channel the user opted into. Stable is the default and ignores prereleases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateChannel {
    #[default]
    Stable,
    Prerelease,
}

impl UpdateChannel {
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Prerelease => "prerelease",
        }
    }

    pub fn from_setting(value: &str) -> Option<Self> {
        match value {
            "stable" => Some(Self::Stable),
            "prerelease" => Some(Self::Prerelease),
            _ => None,
        }
    }
}

/// The channel a build defaults to when the user has not made an explicit choice.
/// A build installed from a prerelease tag keeps receiving prereleases by default;
/// a stable build stays on stable by default. This only decides the *default* —
/// an explicit user preference always overrides it, see [`effective_update_channel`].
pub fn default_channel_for_version(version: &Version) -> UpdateChannel {
    if version.pre.is_empty() {
        UpdateChannel::Stable
    } else {
        UpdateChannel::Prerelease
    }
}

/// Resolves the channel actually used for an update check: an explicit user
/// preference always wins; otherwise falls back to [`default_channel_for_version`]
/// so prerelease installs keep following prereleases without extra configuration.
pub fn effective_update_channel(
    explicit_preference: Option<UpdateChannel>,
    current_version: &Version,
) -> UpdateChannel {
    explicit_preference.unwrap_or_else(|| default_channel_for_version(current_version))
}

/// A newer compatible release than the currently running build.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateInfo {
    pub current_version: Version,
    pub latest_version: Version,
    pub release_url: String,
    pub release_notes: Option<String>,
    pub prerelease: bool,
}

/// State of an update check as surfaced to the UI.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    Failed(String),
}

/// Raw shape of a single entry from the GitHub "list releases" API.
/// Only the fields the update checker needs are declared; unknown fields are ignored by serde.
#[derive(Debug, Clone, Deserialize)]
struct GithubReleaseRaw {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
}

/// A GitHub release with its tag parsed into a proper SemVer version.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseInfo {
    pub tag: String,
    pub version: Version,
    pub prerelease: bool,
    pub url: String,
    pub notes: Option<String>,
    pub published_at: Option<String>,
}

/// Strips a leading `v` from a release tag (e.g. `v0.1.0-rc.3` -> `0.1.0-rc.3`).
pub fn normalize_release_tag(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// Parses a release tag into a SemVer version, normalizing a leading `v` first.
/// Returns `None` for malformed tags instead of failing the whole check.
pub fn parse_release_version(tag: &str) -> Option<Version> {
    Version::parse(normalize_release_tag(tag)).ok()
}

/// Parses the JSON body of the GitHub "list releases" endpoint into release metadata.
/// Draft releases are dropped, and entries with an unparsable tag are skipped rather
/// than aborting the whole batch.
pub fn parse_github_releases(body: &str) -> Result<Vec<ReleaseInfo>, String> {
    let raw: Vec<GithubReleaseRaw> = serde_json::from_str(body)
        .map_err(|error| format!("Resposta inesperada do GitHub: {error}"))?;
    Ok(raw
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = parse_release_version(&release.tag_name)?;
            Some(ReleaseInfo {
                tag: release.tag_name,
                version,
                prerelease: release.prerelease,
                url: release.html_url,
                notes: release.body,
                published_at: release.published_at,
            })
        })
        .collect())
}

/// Picks the newest release compatible with the given channel.
/// Stable ignores prereleases entirely; Prerelease considers both stable and prerelease tags.
pub fn select_latest_release(
    releases: &[ReleaseInfo],
    channel: UpdateChannel,
) -> Option<&ReleaseInfo> {
    releases
        .iter()
        .filter(|release| channel == UpdateChannel::Prerelease || !release.prerelease)
        .max_by(|a, b| a.version.cmp(&b.version))
}

/// Decides whether a compatible release newer than `current_version` exists.
/// Returns `None` when up to date, when the current build is already newer
/// (e.g. ahead of a prerelease line), or when no compatible release was found at all.
pub fn evaluate_update(
    current_version: &Version,
    releases: &[ReleaseInfo],
    channel: UpdateChannel,
) -> Option<UpdateInfo> {
    let latest = select_latest_release(releases, channel)?;
    if latest.version <= *current_version {
        return None;
    }
    Some(UpdateInfo {
        current_version: current_version.clone(),
        latest_version: latest.version.clone(),
        release_url: latest.url.clone(),
        release_notes: latest.notes.clone(),
        prerelease: latest.prerelease,
    })
}

/// Whether the automatic notification banner should be shown for this update,
/// given a version the user previously chose to skip.
pub fn should_notify_update(update: &UpdateInfo, skipped_version: Option<&Version>) -> bool {
    skipped_version != Some(&update.latest_version)
}

/// Minimum time between automatic update checks *within a single already-running
/// process*. It intentionally has no bearing on whether a freshly started process
/// performs its own check — see [`should_check_on_startup`].
pub const UPDATE_CHECK_INTERVAL_SECONDS: i64 = 24 * 60 * 60;

/// Whether enough time has passed since the last automatic check attempt to run
/// another one *in the same process*. Kept for any future in-session periodic
/// recheck; deliberately not consulted when deciding a fresh process's own check.
pub fn update_check_due(last_check_unix: Option<i64>, now_unix: i64) -> bool {
    match last_check_unix {
        None => true,
        Some(last) => now_unix.saturating_sub(last) >= UPDATE_CHECK_INTERVAL_SECONDS,
    }
}

/// Whether a freshly started process should perform its one automatic update check.
/// A new process always checks once when automatic checking is enabled: the
/// persisted "last check" timestamp (see [`update_check_due`]) must never suppress
/// the first check of a new launch, even if a previous process checked minutes ago.
pub fn should_check_on_startup(auto_check_enabled: bool) -> bool {
    auto_check_enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool) -> ReleaseInfo {
        ReleaseInfo {
            tag: tag.to_string(),
            version: parse_release_version(tag).expect("valid test fixture tag"),
            prerelease,
            url: format!("https://github.com/sergiocardoso/flokin-md/releases/tag/{tag}"),
            notes: None,
            published_at: None,
        }
    }

    fn version(value: &str) -> Version {
        Version::parse(value).expect("valid test fixture version")
    }

    #[test]
    fn patch_update_is_available() {
        let releases = vec![release("v0.1.1", false)];
        let update = evaluate_update(&version("0.1.0"), &releases, UpdateChannel::Stable);
        let update = update.expect("update should be available");
        assert_eq!(update.latest_version, version("0.1.1"));
        assert!(!update.prerelease);
    }

    #[test]
    fn same_version_is_up_to_date() {
        let releases = vec![release("v0.1.1", false)];
        let update = evaluate_update(&version("0.1.1"), &releases, UpdateChannel::Stable);
        assert!(update.is_none());
    }

    #[test]
    fn older_latest_release_is_not_an_update() {
        let releases = vec![release("v0.1.9", false)];
        let update = evaluate_update(&version("0.2.0"), &releases, UpdateChannel::Stable);
        assert!(update.is_none());
    }

    #[test]
    fn prerelease_ordering_follows_semver() {
        let releases = vec![release("v0.1.0-rc.3", true)];
        let update = evaluate_update(&version("0.1.0-rc.2"), &releases, UpdateChannel::Prerelease);
        let update = update.expect("rc.3 should be newer than rc.2");
        assert_eq!(update.latest_version, version("0.1.0-rc.3"));
    }

    #[test]
    fn stable_release_is_newer_than_its_own_release_candidates() {
        let releases = vec![release("v0.1.0", false)];
        let update = evaluate_update(&version("0.1.0-rc.3"), &releases, UpdateChannel::Prerelease);
        let update = update.expect("stable 0.1.0 should be newer than rc.3");
        assert_eq!(update.latest_version, version("0.1.0"));
    }

    #[test]
    fn stable_channel_ignores_prerelease_only_releases() {
        let releases = vec![release("v0.2.0-rc.1", true)];
        let update = evaluate_update(&version("0.1.0"), &releases, UpdateChannel::Stable);
        assert!(update.is_none());
    }

    #[test]
    fn prerelease_channel_accepts_newer_prerelease() {
        let releases = vec![release("v0.2.0-rc.1", true)];
        let update = evaluate_update(&version("0.1.0"), &releases, UpdateChannel::Prerelease);
        let update = update.expect("prerelease channel should accept newer rc");
        assert_eq!(update.latest_version, version("0.2.0-rc.1"));
        assert!(update.prerelease);
    }

    #[test]
    fn prerelease_install_defaults_to_prerelease_channel() {
        assert_eq!(
            default_channel_for_version(&version("0.1.0-rc.2")),
            UpdateChannel::Prerelease
        );
        assert_eq!(
            effective_update_channel(None, &version("0.1.0-rc.2")),
            UpdateChannel::Prerelease
        );
    }

    #[test]
    fn stable_install_defaults_to_stable_channel() {
        assert_eq!(
            default_channel_for_version(&version("0.1.0")),
            UpdateChannel::Stable
        );
        assert_eq!(
            effective_update_channel(None, &version("0.1.0")),
            UpdateChannel::Stable
        );
    }

    #[test]
    fn explicit_preference_overrides_the_version_based_default() {
        // A prerelease build whose user explicitly opted into Stable stays on Stable...
        assert_eq!(
            effective_update_channel(Some(UpdateChannel::Stable), &version("0.1.0-rc.2")),
            UpdateChannel::Stable
        );
        // ...and a stable build whose user explicitly opted into Prerelease follows it.
        assert_eq!(
            effective_update_channel(Some(UpdateChannel::Prerelease), &version("0.1.0")),
            UpdateChannel::Prerelease
        );
    }

    #[test]
    fn rc_to_rc_notifies_without_any_channel_configuration() {
        let current = version("0.1.0-rc.2");
        let channel = effective_update_channel(None, &current);
        let releases = vec![release("v0.1.0-rc.3", true)];

        let update = evaluate_update(&current, &releases, channel);

        let update = update.expect("a newer rc should notify a prerelease install by default");
        assert_eq!(update.latest_version, version("0.1.0-rc.3"));
    }

    #[test]
    fn rc_to_stable_notifies_without_any_channel_configuration() {
        let current = version("0.1.0-rc.3");
        let channel = effective_update_channel(None, &current);
        let releases = vec![release("v0.1.0", false)];

        let update = evaluate_update(&current, &releases, channel);

        let update = update.expect("the stable release should notify a prerelease install");
        assert_eq!(update.latest_version, version("0.1.0"));
    }

    #[test]
    fn stable_install_does_not_receive_prerelease_only_release_by_default() {
        let current = version("0.1.0");
        let channel = effective_update_channel(None, &current);
        let releases = vec![release("v0.2.0-rc.1", true)];

        let update = evaluate_update(&current, &releases, channel);

        assert!(update.is_none());
    }

    #[test]
    fn leading_v_is_normalized_before_parsing() {
        assert_eq!(parse_release_version("v1.2.3"), Some(version("1.2.3")));
        assert_eq!(parse_release_version("1.2.3"), Some(version("1.2.3")));
    }

    #[test]
    fn malformed_tag_is_skipped_not_fatal() {
        assert_eq!(parse_release_version("not-a-version"), None);
        assert_eq!(parse_release_version("v"), None);

        let body = r#"[
            {"tag_name": "not-a-version", "prerelease": false, "draft": false, "html_url": "https://example.com/a"},
            {"tag_name": "v1.0.0", "prerelease": false, "draft": false, "html_url": "https://example.com/b"}
        ]"#;
        let releases = parse_github_releases(body).expect("parsing itself should not fail");
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version, version("1.0.0"));
    }

    #[test]
    fn no_releases_means_no_update() {
        let releases: Vec<ReleaseInfo> = Vec::new();
        let update = evaluate_update(&version("0.1.0"), &releases, UpdateChannel::Stable);
        assert!(update.is_none());
    }

    #[test]
    fn draft_releases_are_excluded() {
        let body = r#"[
            {"tag_name": "v9.0.0", "prerelease": false, "draft": true, "html_url": "https://example.com/draft"}
        ]"#;
        let releases = parse_github_releases(body).expect("parsing should succeed");
        assert!(releases.is_empty());
    }

    #[test]
    fn skipped_version_suppresses_notification() {
        let update = UpdateInfo {
            current_version: version("0.1.0"),
            latest_version: version("0.1.1"),
            release_url: "https://example.com".to_string(),
            release_notes: None,
            prerelease: false,
        };
        assert!(!should_notify_update(&update, Some(&version("0.1.1"))));
    }

    #[test]
    fn newer_release_after_skip_still_notifies() {
        let update = UpdateInfo {
            current_version: version("0.1.0"),
            latest_version: version("0.1.2"),
            release_url: "https://example.com".to_string(),
            release_notes: None,
            prerelease: false,
        };
        assert!(should_notify_update(&update, Some(&version("0.1.1"))));
    }

    #[test]
    fn no_skip_always_notifies() {
        let update = UpdateInfo {
            current_version: version("0.1.0"),
            latest_version: version("0.1.1"),
            release_url: "https://example.com".to_string(),
            release_notes: None,
            prerelease: false,
        };
        assert!(should_notify_update(&update, None));
    }

    #[test]
    fn malformed_api_response_produces_controlled_error() {
        let result = parse_github_releases("{ this is not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn unexpected_json_shape_produces_controlled_error() {
        let result = parse_github_releases(r#"{"message": "Not Found"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn update_check_due_respects_interval() {
        let now = 1_000_000_i64;
        assert!(update_check_due(None, now));
        assert!(!update_check_due(Some(now - 60), now));
        assert!(update_check_due(
            Some(now - UPDATE_CHECK_INTERVAL_SECONDS),
            now
        ));
        assert!(update_check_due(
            Some(now - UPDATE_CHECK_INTERVAL_SECONDS - 1),
            now
        ));
    }

    #[test]
    fn startup_check_runs_when_auto_check_is_enabled() {
        assert!(should_check_on_startup(true));
    }

    #[test]
    fn startup_check_is_skipped_when_auto_check_is_disabled() {
        assert!(!should_check_on_startup(false));
    }

    #[test]
    fn startup_check_ignores_a_recent_persisted_timestamp() {
        // Under the *in-process* interval rule a check made seconds ago would not
        // be due again yet...
        let now = 1_000_000_i64;
        let checked_moments_ago = Some(now - 60);
        assert!(!update_check_due(checked_moments_ago, now));

        // ...but a fresh process launch must still run its own check regardless,
        // because `should_check_on_startup` never looks at that timestamp at all.
        assert!(should_check_on_startup(true));
    }

    #[test]
    fn rc2_to_rc3_with_no_channel_preference_would_populate_the_global_banner() {
        let current = version("0.1.0-rc.2");
        let channel = effective_update_channel(None, &current);
        let releases = vec![release("v0.1.0-rc.3", true)];

        let update = evaluate_update(&current, &releases, channel).expect(
            "rc.3 should be discovered for a prerelease install with no channel preference",
        );

        assert_eq!(update.latest_version, version("0.1.0-rc.3"));
        assert!(
            should_notify_update(&update, None),
            "with no skipped version, the global banner must be shown"
        );
    }
}
