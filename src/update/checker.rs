use crate::core::{RacError, RacResult};
use crate::update::http::{
    configure_request, connect, handle_status_code, open_request, open_session, parse_url,
    query_status_code, read_response_body, send_request,
};
use crate::update::version::Version;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;
use windows::Win32::Networking::WinHttp::WinHttpSetTimeouts;

const MAX_RETRIES: u8 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: Version,
    pub download_url: String,
    pub release_name: String,
    pub release_notes: String,
    pub asset_size: u64,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: String,
    body: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Clone)]
pub struct UpdateChecker {
    owner: String,
    repo: String,
    current_version: Version,
}

impl UpdateChecker {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            current_version: Version::current(),
        }
    }

    pub fn check_for_updates(&self) -> RacResult<Option<ReleaseInfo>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        );

        let mut attempts = 0;
        let mut last_error = None;

        while attempts < MAX_RETRIES {
            match self.fetch_release_info(&url) {
                Ok(release_data) => {
                    let latest_version = Version::parse(&release_data.tag_name).map_err(|e| {
                        RacError::UpdateError(format!("Invalid version in release: {}", e))
                    })?;

                    if !latest_version.is_newer_than(&self.current_version) {
                        return Ok(None);
                    }

                    let exe_asset = release_data
                        .assets
                        .iter()
                        .find(|a| a.name.ends_with(".exe"))
                        .ok_or_else(|| {
                            RacError::UpdateError(
                                "No Windows executable found in release".to_string(),
                            )
                        })?;

                    return Ok(Some(ReleaseInfo {
                        version: latest_version,
                        download_url: exe_asset.browser_download_url.clone(),
                        release_name: release_data.name,
                        release_notes: release_data.body,
                        asset_size: exe_asset.size,
                    }));
                }
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);

                    if let Some(ref err) = last_error
                        && (err.to_string().contains("404") || err.to_string().contains("403"))
                    {
                        return Err(last_error.unwrap());
                    }

                    if attempts < MAX_RETRIES {
                        let delay = INITIAL_RETRY_DELAY_MS * 2u64.pow((attempts - 1) as u32);
                        thread::sleep(Duration::from_millis(delay));
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            RacError::UpdateError("Failed to check for updates after retries".to_string())
        }))
    }

    fn fetch_release_info(&self, url: &str) -> RacResult<GithubRelease> {
        let user_agent = format!("RAC-Updater/{}", env!("CARGO_PKG_VERSION"));

        let url_parts = parse_url(url, 1024)?;

        let session = open_session(&user_agent)?;

        let connection = connect(&session, &url_parts.host, url_parts.port)?;

        let request = open_request(&connection, &url_parts.path)?;

        unsafe {
            WinHttpSetTimeouts(request.as_ptr(), 30000, 30000, 30000, 30000)
                .map_err(|e| RacError::UpdateError(format!("Failed to set timeout: {:?}", e)))?;
        }

        configure_request(&request, &user_agent)?;

        send_request(&request)?;

        let status_code = query_status_code(&request)?;
        handle_status_code(status_code, "GitHub API")?;

        let response_data = read_response_body(&request)?;

        serde_json::from_slice(&response_data)
            .map_err(|e| RacError::UpdateError(format!("Failed to parse JSON: {}", e)))
    }

    pub fn current_version(&self) -> &Version {
        &self.current_version
    }
}
