use crate::core::{RacError, RacResult};
use crate::update::version::Version;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

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

struct WinHttpHandle(*mut std::ffi::c_void);

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = windows::Win32::Networking::WinHttp::WinHttpCloseHandle(self.0);
            }
        }
    }
}

impl WinHttpHandle {
    fn new(handle: *mut std::ffi::c_void) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0
    }
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
        use windows::Win32::Networking::WinHttp::*;
        use windows::core::{HSTRING, PCWSTR};

        unsafe {
            let user_agent = format!("RAC-Updater/{}", env!("CARGO_PKG_VERSION"));
            let session = WinHttpHandle::new(WinHttpOpen(
                &HSTRING::from(user_agent),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            ))
            .ok_or_else(|| {
                let error_code = windows::Win32::Foundation::GetLastError().0;
                RacError::UpdateError(format!(
                    "Failed to open WinHTTP session (error code: 0x{:X})",
                    error_code
                ))
            })?;

            let protocols: u32 =
                WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2 | WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;
            let _ = WinHttpSetOption(
                Some(session.as_ptr() as *const _),
                WINHTTP_OPTION_SECURE_PROTOCOLS,
                Some(&protocols.to_ne_bytes()),
            );

            let (host, path, port) = parse_url(url)?;

            let connect = WinHttpHandle::new(WinHttpConnect(
                session.as_ptr(),
                &HSTRING::from(host.as_str()),
                port,
                0,
            ))
            .ok_or_else(|| {
                let error_code = windows::Win32::Foundation::GetLastError().0;
                RacError::UpdateError(format!(
                    "Failed to connect to {} (error code: 0x{:X})",
                    host, error_code
                ))
            })?;

            let request = WinHttpHandle::new(WinHttpOpenRequest(
                connect.as_ptr(),
                &HSTRING::from("GET"),
                &HSTRING::from(path.as_str()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            ))
            .ok_or_else(|| {
                let error_code = windows::Win32::Foundation::GetLastError().0;
                RacError::UpdateError(format!(
                    "Failed to open request (error code: 0x{:X})",
                    error_code
                ))
            })?;

            WinHttpSetTimeouts(request.as_ptr(), 30000, 30000, 30000, 30000)
                .map_err(|e| RacError::UpdateError(format!("Failed to set timeout: {:?}", e)))?;

            let redirect_policy: u32 = WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS;
            let _ = WinHttpSetOption(
                Some(request.as_ptr() as *const _),
                WINHTTP_OPTION_REDIRECT_POLICY,
                Some(&redirect_policy.to_ne_bytes()),
            );

            let decompression: u32 =
                WINHTTP_DECOMPRESSION_FLAG_GZIP | WINHTTP_DECOMPRESSION_FLAG_DEFLATE;
            let _ = WinHttpSetOption(
                Some(request.as_ptr() as *const _),
                WINHTTP_OPTION_DECOMPRESSION,
                Some(&decompression.to_ne_bytes()),
            );

            let headers = HSTRING::from(format!(
                "User-Agent: RAC-Updater/{}\r\n",
                env!("CARGO_PKG_VERSION")
            ));
            let _ = WinHttpAddRequestHeaders(request.as_ptr(), &headers, WINHTTP_ADDREQ_FLAG_ADD);

            WinHttpSendRequest(request.as_ptr(), None, None, 0, 0, 0).map_err(|e| {
                let error_code = windows::Win32::Foundation::GetLastError().0;
                RacError::UpdateError(format!(
                    "Failed to send request (error code: 0x{:X}, details: {:?})",
                    error_code, e
                ))
            })?;

            WinHttpReceiveResponse(request.as_ptr(), std::ptr::null_mut()).map_err(|e| {
                let error_code = windows::Win32::Foundation::GetLastError().0;
                RacError::UpdateError(format!(
                    "Failed to receive response (error code: 0x{:X}, details: {:?})",
                    error_code, e
                ))
            })?;

            let status_code = query_status_code(request.as_ptr())?;

            match status_code {
                200 => {}
                304 => {
                    return Err(RacError::UpdateError("Not modified (304)".to_string()));
                }
                403 => {
                    return Err(RacError::UpdateError(
                        "GitHub API access forbidden (403). Possible rate limit or authentication issue.".to_string()
                    ));
                }
                404 => {
                    return Err(RacError::UpdateError(
                        "Release not found (404). Repository may not exist or no releases published.".to_string()
                    ));
                }
                429 => {
                    return Err(RacError::UpdateError(
                        "Rate limited (429). GitHub API limit exceeded. Try again later."
                            .to_string(),
                    ));
                }
                500..=599 => {
                    return Err(RacError::UpdateError(format!(
                        "GitHub server error ({}). Service temporarily unavailable.",
                        status_code
                    )));
                }
                _ => {
                    return Err(RacError::UpdateError(format!(
                        "HTTP error: {}",
                        status_code
                    )));
                }
            }

            let response_data = read_response_body(request.as_ptr())?;

            serde_json::from_slice(&response_data)
                .map_err(|e| RacError::UpdateError(format!("Failed to parse JSON: {}", e)))
        }
    }

    pub fn current_version(&self) -> &Version {
        &self.current_version
    }
}

fn parse_url(url: &str) -> RacResult<(String, String, u16)> {
    use windows::Win32::Networking::WinHttp::*;

    unsafe {
        let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let mut components: URL_COMPONENTS = std::mem::zeroed();
        components.dwStructSize = size_of::<URL_COMPONENTS>() as u32;

        let mut host_buffer = vec![0u16; 256];
        let mut path_buffer = vec![0u16; 1024];

        components.lpszHostName = windows::core::PWSTR(host_buffer.as_mut_ptr());
        components.dwHostNameLength = host_buffer.len() as u32;
        components.lpszUrlPath = windows::core::PWSTR(path_buffer.as_mut_ptr());
        components.dwUrlPathLength = path_buffer.len() as u32;

        WinHttpCrackUrl(&url_wide, 0, &mut components)
            .map_err(|_| RacError::UpdateError("Failed to parse URL".into()))?;

        let host = String::from_utf16_lossy(&host_buffer[..components.dwHostNameLength as usize]);
        let path = String::from_utf16_lossy(&path_buffer[..components.dwUrlPathLength as usize]);

        Ok((host, path, components.nPort))
    }
}

fn query_status_code(request: *mut std::ffi::c_void) -> RacResult<u32> {
    use windows::Win32::Networking::WinHttp::*;
    use windows::core::PCWSTR;

    unsafe {
        let mut status_code: u32 = 0;
        let mut size = size_of::<u32>() as u32;

        WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut status_code as *mut u32 as *mut _),
            &mut size,
            std::ptr::null_mut(),
        )
        .map_err(|_| RacError::UpdateError("Failed to query status".into()))?;

        Ok(status_code)
    }
}

fn read_response_body(request: *mut std::ffi::c_void) -> RacResult<Vec<u8>> {
    use windows::Win32::Networking::WinHttp::WinHttpReadData;

    let mut response_data = Vec::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let mut bytes_read: u32 = 0;
        let result = unsafe {
            WinHttpReadData(
                request,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut bytes_read,
            )
        };

        if result.is_err() || bytes_read == 0 {
            break;
        }
        response_data.extend_from_slice(&buffer[..bytes_read as usize]);
    }

    Ok(response_data)
}
