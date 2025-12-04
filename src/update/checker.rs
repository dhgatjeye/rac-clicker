use crate::core::{RacResult, RacError};
use crate::update::version::Version;
use serde::{Deserialize, Serialize};

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

        let release_data = self.fetch_release_info(&url)?;
        let latest_version = Version::parse(&release_data.tag_name)
            .map_err(|e| RacError::UpdateError(format!("Invalid version in release: {}", e)))?;

        if !latest_version.is_newer_than(&self.current_version) {
            return Ok(None);
        }

        let exe_asset = release_data
            .assets
            .iter()
            .find(|a| a.name.ends_with(".exe"))
            .ok_or_else(|| RacError::UpdateError("No Windows executable found in release".to_string()))?;

        Ok(Some(ReleaseInfo {
            version: latest_version,
            download_url: exe_asset.browser_download_url.clone(),
            release_name: release_data.name,
            release_notes: release_data.body,
            asset_size: exe_asset.size
        }))
    }

    fn fetch_release_info(&self, url: &str) -> RacResult<GithubRelease> {
        use windows::Win32::Networking::WinHttp::*;
        use windows::core::{PCWSTR, HSTRING};

        unsafe {
            let session = WinHttpOpen(
                &HSTRING::from("RAC-Updater/1.0"),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            );

            if session.is_null() {
                return Err(RacError::UpdateError("Failed to open WinHTTP session".to_string()));
            }

            let protocols: u32 = WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;
            let protocols_bytes = protocols.to_ne_bytes();
            let _ = WinHttpSetOption(
                Some(session as *const std::ffi::c_void),
                WINHTTP_OPTION_SECURE_PROTOCOLS,
                Some(&protocols_bytes),
            );

            let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
            let mut url_components: URL_COMPONENTS = std::mem::zeroed();
            url_components.dwStructSize = size_of::<URL_COMPONENTS>() as u32;

            let mut host_buffer = vec![0u16; 256];
            let mut path_buffer = vec![0u16; 1024];

            url_components.lpszHostName = windows::core::PWSTR(host_buffer.as_mut_ptr());
            url_components.dwHostNameLength = host_buffer.len() as u32;
            url_components.lpszUrlPath = windows::core::PWSTR(path_buffer.as_mut_ptr());
            url_components.dwUrlPathLength = path_buffer.len() as u32;

            if WinHttpCrackUrl(&url_wide, 0, &mut url_components).is_err() {
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to parse URL".to_string()));
            }

            let host_name = HSTRING::from_wide(&host_buffer[..url_components.dwHostNameLength as usize]);

            let connect = WinHttpConnect(
                session,
                &host_name,
                url_components.nPort,
                0,
            );

            if connect.is_null() {
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to connect to server".to_string()));
            }

            let path = HSTRING::from_wide(&path_buffer[..url_components.dwUrlPathLength as usize]);

            let request = WinHttpOpenRequest(
                connect,
                &HSTRING::from("GET"),
                &path,
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            );

            if request.is_null() {
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to open request".to_string()));
            }

            if WinHttpSetTimeouts(request, 30000, 30000, 30000, 30000).is_err() {
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to set timeout".to_string()));
            }

            let headers = HSTRING::from("User-Agent: RAC-Updater/1.0\r\n");
            let _ = WinHttpAddRequestHeaders(
                request,
                &headers,
                WINHTTP_ADDREQ_FLAG_ADD,
            );

            if WinHttpSendRequest(
                request,
                None,
                None,
                0,
                0,
                0,
            ).is_err() {
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to send request".to_string()));
            }

            if WinHttpReceiveResponse(request, std::ptr::null_mut()).is_err() {
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to receive response".to_string()));
            }

            let mut status_code: u32 = 0;
            let mut size = size_of::<u32>() as u32;
            if WinHttpQueryHeaders(
                request,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(&mut status_code as *mut u32 as *mut _),
                &mut size,
                std::ptr::null_mut(),
            ).is_err() {
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to query status".to_string()));
            }

            if status_code != 200 {
                return Err(RacError::UpdateError(format!("HTTP error: {}", status_code)));
            }

            let mut response_data = Vec::new();
            let mut buffer = vec![0u8; 8192];

            loop {
                let mut bytes_read: u32 = 0;
                if WinHttpReadData(
                    request,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as u32,
                    &mut bytes_read,
                ).is_err() || bytes_read == 0 {
                    break;
                }
                response_data.extend_from_slice(&buffer[..bytes_read as usize]);
            }

            let _ = WinHttpCloseHandle(request);
            let _ = WinHttpCloseHandle(connect);
            let _ = WinHttpCloseHandle(session);

            let release: GithubRelease = serde_json::from_slice(&response_data)
                .map_err(|e| RacError::UpdateError(format!("Failed to parse JSON: {}", e)))?;

            Ok(release)
        }
    }

    pub fn current_version(&self) -> &Version {
        &self.current_version
    }
}