use crate::core::{RacError, RacResult};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const MAX_DOWNLOAD_RETRIES: u8 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 2000;

pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

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
pub struct Downloader;

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Downloader {
    pub fn new() -> Self {
        Self
    }

    pub fn download(
        &self,
        url: &str,
        dest_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> RacResult<PathBuf> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < MAX_DOWNLOAD_RETRIES {
            match self.download_internal(url, dest_path, progress_callback.clone()) {
                Ok(path) => return Ok(path),
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);

                    if let Some(ref err) = last_error {
                        let err_str = err.to_string();
                        if err_str.contains("404") || err_str.contains("403") {
                            let _ = std::fs::remove_file(dest_path);
                            return Err(last_error.unwrap());
                        }
                    }

                    if attempts < MAX_DOWNLOAD_RETRIES {
                        let delay = INITIAL_RETRY_DELAY_MS * 2u64.pow((attempts - 1) as u32);
                        thread::sleep(Duration::from_millis(delay));

                        let _ = std::fs::remove_file(dest_path);
                    }
                }
            }
        }

        let _ = std::fs::remove_file(dest_path);
        Err(last_error.unwrap_or_else(|| {
            RacError::UpdateError("Failed to download after retries".to_string())
        }))
    }

    fn download_internal(
        &self,
        url: &str,
        dest_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> RacResult<PathBuf> {
        use windows::Win32::Networking::WinHttp::*;
        use windows::core::{HSTRING, PCWSTR};

        unsafe {
            let user_agent = format!("RAC-Downloader/{}", env!("CARGO_PKG_VERSION"));
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
                    "Failed to initialize WinHTTP (error code: 0x{:X})",
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

            WinHttpSetTimeouts(request.as_ptr(), 30000, 60000, 60000, 60000)
                .map_err(|e| RacError::UpdateError(format!("Failed to set timeout: {:?}", e)))?;

            let redirect_policy: u32 = WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS;
            let _ = WinHttpSetOption(
                Some(request.as_ptr() as *const _),
                WINHTTP_OPTION_REDIRECT_POLICY,
                Some(&redirect_policy.to_ne_bytes()),
            );

            let max_redirects: u32 = 10;
            let _ = WinHttpSetOption(
                Some(request.as_ptr() as *const _),
                WINHTTP_OPTION_MAX_HTTP_AUTOMATIC_REDIRECTS,
                Some(&max_redirects.to_ne_bytes()),
            );

            let decompression: u32 =
                WINHTTP_DECOMPRESSION_FLAG_GZIP | WINHTTP_DECOMPRESSION_FLAG_DEFLATE;
            let _ = WinHttpSetOption(
                Some(request.as_ptr() as *const _),
                WINHTTP_OPTION_DECOMPRESSION,
                Some(&decompression.to_ne_bytes()),
            );

            let headers = HSTRING::from(format!(
                "User-Agent: RAC-Downloader/{}\r\n",
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
                301 | 302 | 303 | 307 | 308 => {
                    return Err(RacError::UpdateError(format!(
                        "Redirect not handled properly ({}). This shouldn't happen with auto-redirect enabled.",
                        status_code
                    )));
                }
                403 => {
                    return Err(RacError::UpdateError(
                        "Download forbidden (403). File may not be publicly accessible."
                            .to_string(),
                    ));
                }
                404 => {
                    return Err(RacError::UpdateError(
                        "Download file not found (404). URL may be invalid.".to_string(),
                    ));
                }
                429 => {
                    return Err(RacError::UpdateError(
                        "Rate limited (429). Too many download requests. Try again later."
                            .to_string(),
                    ));
                }
                500..=599 => {
                    return Err(RacError::UpdateError(format!(
                        "Server error ({}). Download service temporarily unavailable.",
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

            let content_length = query_content_length(request.as_ptr());

            let result = download_to_file(
                request.as_ptr(),
                dest_path,
                content_length,
                progress_callback,
            );

            if result.is_err() {
                let _ = std::fs::remove_file(dest_path);
            }

            result
        }
    }
}

fn parse_url(url: &str) -> RacResult<(String, String, u16)> {
    use windows::Win32::Networking::WinHttp::*;

    unsafe {
        let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let mut components: URL_COMPONENTS = std::mem::zeroed();
        components.dwStructSize = size_of::<URL_COMPONENTS>() as u32;

        let mut host_buffer = vec![0u16; 256];
        let mut path_buffer = vec![0u16; 2048];

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

fn query_content_length(request: *mut std::ffi::c_void) -> u64 {
    use windows::Win32::Networking::WinHttp::*;
    use windows::core::PCWSTR;

    unsafe {
        let mut content_length: u64 = 0;
        let mut size = size_of::<u64>() as u32;

        let _ = WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut content_length as *mut u64 as *mut _),
            &mut size,
            std::ptr::null_mut(),
        );

        content_length
    }
}

fn download_to_file(
    request: *mut std::ffi::c_void,
    dest_path: &Path,
    content_length: u64,
    progress_callback: Option<ProgressCallback>,
) -> RacResult<PathBuf> {
    use windows::Win32::Networking::WinHttp::WinHttpReadData;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest_path)
        .map_err(|e| RacError::UpdateError(format!("Failed to create file: {}", e)))?;

    let mut buffer = vec![0u8; 16384];
    let mut total_downloaded: u64 = 0;

    loop {
        let mut bytes_read: u32 = 0;

        let read_result = unsafe {
            WinHttpReadData(
                request,
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut bytes_read,
            )
        };

        if read_result.is_err() || bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read as usize])
            .map_err(|e| RacError::UpdateError(format!("Write failed: {}", e)))?;

        total_downloaded += bytes_read as u64;

        if let Some(ref callback) = progress_callback {
            callback(total_downloaded, content_length);
        }
    }

    file.flush()
        .map_err(|e| RacError::UpdateError(format!("Flush failed: {}", e)))?;

    if content_length > 0 && total_downloaded != content_length {
        return Err(RacError::UpdateError(format!(
            "Download incomplete: expected {} bytes, got {}",
            content_length, total_downloaded
        )));
    }

    Ok(dest_path.to_path_buf())
}
