use crate::core::{RacResult, RacError};
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

pub struct Downloader {
    progress: Arc<AtomicU64>,
    total_size: Arc<AtomicU64>,
    cancelled: Arc<AtomicBool>,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(AtomicU64::new(0)),
            total_size: Arc::new(AtomicU64::new(0)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn download(
        &self,
        url: &str,
        dest_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> RacResult<PathBuf> {
        use windows::Win32::Networking::WinHttp::*;
        use windows::core::{PCWSTR, HSTRING};

        self.progress.store(0, Ordering::Release);
        self.total_size.store(0, Ordering::Release);
        self.cancelled.store(false, Ordering::Release);

        unsafe {
            let session = WinHttpOpen(
                &HSTRING::from("RAC-Downloader/1.0"),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            );

            if session.is_null() {
                return Err(RacError::UpdateError("Failed to initialize WinHTTP".to_string()));
            }

            let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
            let mut url_components: URL_COMPONENTS = std::mem::zeroed();
            url_components.dwStructSize = size_of::<URL_COMPONENTS>() as u32;

            let mut host_buffer = vec![0u16; 256];
            let mut path_buffer = vec![0u16; 2048];

            url_components.lpszHostName = windows::core::PWSTR(host_buffer.as_mut_ptr());
            url_components.dwHostNameLength = host_buffer.len() as u32;
            url_components.lpszUrlPath = windows::core::PWSTR(path_buffer.as_mut_ptr());
            url_components.dwUrlPathLength = path_buffer.len() as u32;

            if WinHttpCrackUrl(&url_wide, 0, &mut url_components).is_err() {
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to parse URL".to_string()));
            }

            let host_name = HSTRING::from_wide(&host_buffer[..url_components.dwHostNameLength as usize]);

            let connect = WinHttpConnect(session, &host_name, url_components.nPort, 0);

            if connect.is_null() {
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to connect".to_string()));
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

            if WinHttpSetTimeouts(request, 30000, 60000, 60000, 60000).is_err() {
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError("Failed to set timeout".to_string()));
            }

            if WinHttpSendRequest(request, None, None, 0, 0, 0).is_err() {
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
                let _ = WinHttpCloseHandle(request);
                let _ = WinHttpCloseHandle(connect);
                let _ = WinHttpCloseHandle(session);
                return Err(RacError::UpdateError(format!("HTTP error: {}", status_code)));
            }

            let mut content_length: u64 = 0;
            let mut cl_size = size_of::<u64>() as u32;
            let _ = WinHttpQueryHeaders(
                request,
                WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(&mut content_length as *mut u64 as *mut _),
                &mut cl_size,
                std::ptr::null_mut(),
            );

            self.total_size.store(content_length, Ordering::Release);

            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(dest_path)
                .map_err(|e| RacError::UpdateError(format!("Failed to create file: {}", e)))?;

            let mut buffer = vec![0u8; 16384];
            let mut total_downloaded: u64 = 0;

            loop {
                if self.cancelled.load(Ordering::Acquire) {
                    let _ = WinHttpCloseHandle(request);
                    let _ = WinHttpCloseHandle(connect);
                    let _ = WinHttpCloseHandle(session);
                    let _ = std::fs::remove_file(dest_path);
                    return Err(RacError::UpdateError("Download cancelled".to_string()));
                }

                let mut bytes_read: u32 = 0;
                if WinHttpReadData(
                    request,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as u32,
                    &mut bytes_read,
                ).is_err() || bytes_read == 0 {
                    break;
                }

                file.write_all(&buffer[..bytes_read as usize])
                    .map_err(|e| RacError::UpdateError(format!("Write failed: {}", e)))?;

                total_downloaded += bytes_read as u64;
                self.progress.store(total_downloaded, Ordering::Release);

                if let Some(ref callback) = progress_callback {
                    callback(total_downloaded, content_length);
                }
            }

            file.flush()
                .map_err(|e| RacError::UpdateError(format!("Flush failed: {}", e)))?;

            let _ = WinHttpCloseHandle(request);
            let _ = WinHttpCloseHandle(connect);
            let _ = WinHttpCloseHandle(session);

            if content_length > 0 && total_downloaded != content_length {
                let _ = std::fs::remove_file(dest_path);
                return Err(RacError::UpdateError(format!(
                    "Download incomplete: expected {} bytes, got {}",
                    content_length, total_downloaded
                )));
            }

            Ok(dest_path.to_path_buf())
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn get_progress(&self) -> (u64, u64) {
        (
            self.progress.load(Ordering::Acquire),
            self.total_size.load(Ordering::Acquire),
        )
    }

    pub fn get_percentage(&self) -> f32 {
        let (current, total) = self.get_progress();
        if total == 0 {
            return 0.0;
        }
        (current as f32 / total as f32) * 100.0
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new()
    }
}

pub fn verify_checksum(file_path: &Path, expected_checksum: &str) -> RacResult<bool> {
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(file_path)
        .map_err(|e| RacError::UpdateError(format!("Failed to open file: {}", e)))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| RacError::UpdateError(format!("Read failed: {}", e)))?;

    let calculated = calculate_sha256(&buffer)?;
    Ok(calculated.eq_ignore_ascii_case(expected_checksum))
}

fn calculate_sha256(data: &[u8]) -> RacResult<String> {
    use windows::Win32::Security::Cryptography::{
        BCryptOpenAlgorithmProvider, BCryptCloseAlgorithmProvider,
        BCryptCreateHash, BCryptFinishHash,
        BCryptDestroyHash, BCryptGetProperty,
        BCRYPT_SHA256_ALGORITHM, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
        BCRYPT_ALG_HANDLE, BCRYPT_HASH_HANDLE,
    };
    use windows::core::PCWSTR;

    unsafe {
        let mut alg_handle = BCRYPT_ALG_HANDLE::default();
        if BCryptOpenAlgorithmProvider(
            &mut alg_handle,
            BCRYPT_SHA256_ALGORITHM,
            PCWSTR::null(),
            BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
        ).is_err() {
            return Err(RacError::UpdateError("Crypto init failed".to_string()));
        }
        
        let mut hash_obj_size_buf = [0u8; 4];
        let mut result_len: u32 = 0;
        if BCryptGetProperty(
            alg_handle.into(),
            windows::core::w!("ObjectLength"),
            Some(&mut hash_obj_size_buf),
            &mut result_len,
            0,
        ).is_err() {
            let _ = BCryptCloseAlgorithmProvider(alg_handle, 0);
            return Err(RacError::UpdateError("Get property failed".to_string()));
        }

        let hash_obj_size = u32::from_le_bytes(hash_obj_size_buf);
        let mut hash_object = vec![0u8; hash_obj_size as usize];
        let mut hash_handle = BCRYPT_HASH_HANDLE::default();

        if BCryptCreateHash(
            alg_handle.into(),
            &mut hash_handle,
            Some(&mut hash_object),
            Some(data),
            0,
        ).is_err() {
            let _ = BCryptCloseAlgorithmProvider(alg_handle, 0);
            return Err(RacError::UpdateError("Create hash failed".to_string()));
        }

        let mut hash_value = vec![0u8; 32];
        if BCryptFinishHash(hash_handle.into(), &mut hash_value, 0).is_err() {
            let _ = BCryptDestroyHash(hash_handle.into());
            let _ = BCryptCloseAlgorithmProvider(alg_handle, 0);
            return Err(RacError::UpdateError("Finish hash failed".to_string()));
        }

        let _ = BCryptDestroyHash(hash_handle.into());
        let _ = BCryptCloseAlgorithmProvider(alg_handle, 0);
        
        Ok(hash_value.iter()
            .map(|b| format!("{:02x}", b))
            .collect())
    }
}