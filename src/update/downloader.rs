use crate::core::{RacError, RacResult};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
        use windows::Win32::Networking::WinHttp::*;
        use windows::core::{HSTRING, PCWSTR};

        unsafe {
            let session = WinHttpHandle::new(WinHttpOpen(
                &HSTRING::from("RAC-Downloader/1.0"),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            ))
            .ok_or_else(|| RacError::UpdateError("Failed to initialize WinHTTP".into()))?;

            let protocols: u32 = WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;
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
            .ok_or_else(|| RacError::UpdateError("Failed to connect".into()))?;

            let request = WinHttpHandle::new(WinHttpOpenRequest(
                connect.as_ptr(),
                &HSTRING::from("GET"),
                &HSTRING::from(path.as_str()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            ))
            .ok_or_else(|| RacError::UpdateError("Failed to open request".into()))?;

            WinHttpSetTimeouts(request.as_ptr(), 30000, 60000, 60000, 60000)
                .map_err(|_| RacError::UpdateError("Failed to set timeout".into()))?;

            WinHttpSendRequest(request.as_ptr(), None, None, 0, 0, 0)
                .map_err(|_| RacError::UpdateError("Failed to send request".into()))?;

            WinHttpReceiveResponse(request.as_ptr(), std::ptr::null_mut())
                .map_err(|_| RacError::UpdateError("Failed to receive response".into()))?;

            let status_code = query_status_code(request.as_ptr())?;
            if status_code != 200 {
                return Err(RacError::UpdateError(format!(
                    "HTTP error: {}",
                    status_code
                )));
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
