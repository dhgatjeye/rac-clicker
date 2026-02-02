use crate::core::{RacError, RacResult};
use crate::update::http::{
    configure_request, connect, handle_status_code, open_request, open_session, parse_url,
    query_content_length, query_status_code, send_request,
};
use crate::update::security::{create_file_exclusively, is_reparse_point};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::Networking::WinHttp::{
    WINHTTP_OPTION_MAX_HTTP_AUTOMATIC_REDIRECTS, WinHttpReadData, WinHttpSetOption,
    WinHttpSetTimeouts,
};

const MAX_DOWNLOAD_RETRIES: u8 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 2000;

pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

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
        let user_agent = format!("RAC-Downloader/{}", env!("CARGO_PKG_VERSION"));

        let url_parts = parse_url(url, 2048)?;

        let session = open_session(&user_agent)?;

        let connection = connect(&session, &url_parts.host, url_parts.port)?;

        let request = open_request(&connection, &url_parts.path)?;

        unsafe {
            WinHttpSetTimeouts(request.as_ptr(), 30000, 60000, 60000, 60000)
                .map_err(|e| RacError::UpdateError(format!("Failed to set timeout: {:?}", e)))?;

            let max_redirects: u32 = 10;
            let _ = WinHttpSetOption(
                Some(request.as_ptr() as *const _),
                WINHTTP_OPTION_MAX_HTTP_AUTOMATIC_REDIRECTS,
                Some(&max_redirects.to_ne_bytes()),
            );
        }

        configure_request(&request, &user_agent)?;

        send_request(&request)?;

        let status_code = query_status_code(&request)?;
        handle_status_code(status_code, "Download")?;

        let content_length = query_content_length(&request);

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

fn download_to_file(
    request: *mut std::ffi::c_void,
    dest_path: &Path,
    content_length: u64,
    progress_callback: Option<ProgressCallback>,
) -> RacResult<PathBuf> {
    let mut file = create_file_exclusively(dest_path)?;

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

    file.sync_all()
        .map_err(|e| RacError::UpdateError(format!("Sync failed: {}", e)))?;

    if content_length > 0 && total_downloaded != content_length {
        return Err(RacError::UpdateError(format!(
            "Download incomplete: expected {} bytes, got {}",
            content_length, total_downloaded
        )));
    }

    if is_reparse_point(dest_path) {
        return Err(RacError::UpdateError(format!(
            "Downloaded file '{}' became a reparse point. Update aborted.",
            dest_path.display()
        )));
    }

    Ok(dest_path.to_path_buf())
}
