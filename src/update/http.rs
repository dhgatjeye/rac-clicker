use crate::core::{RacError, RacResult};
use windows::Win32::Networking::WinHttp::{
    URL_COMPONENTS, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ADDREQ_FLAG_ADD,
    WINHTTP_DECOMPRESSION_FLAG_DEFLATE, WINHTTP_DECOMPRESSION_FLAG_GZIP, WINHTTP_FLAG_SECURE,
    WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2, WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3,
    WINHTTP_OPTION_DECOMPRESSION, WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL,
    WINHTTP_OPTION_REDIRECT_POLICY, WINHTTP_OPTION_REDIRECT_POLICY_ALWAYS,
    WINHTTP_OPTION_SECURE_PROTOCOLS, WINHTTP_PROTOCOL_FLAG_HTTP2, WINHTTP_QUERY_CONTENT_LENGTH,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpAddRequestHeaders,
    WinHttpCloseHandle, WinHttpConnect, WinHttpCrackUrl, WinHttpOpen, WinHttpOpenRequest,
    WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetOption,
};
use windows::core::{HSTRING, PCWSTR};

const WINHTTP_OPTION_ENABLE_FEATURE: u32 = 79;
const WINHTTP_ENABLE_SSL_REVOCATION: u32 = 0x00000001;
const ERROR_WINHTTP_SECURE_CERT_REV_FAILED: u32 = 12057;
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

pub struct WinHttpHandle(*mut std::ffi::c_void);

impl WinHttpHandle {
    #[inline]
    pub fn new(handle: *mut std::ffi::c_void) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self(handle))
        }
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0
    }
}

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
}

pub struct UrlComponents {
    pub host: String,
    pub path: String,
    pub port: u16,
}

pub fn parse_url(url: &str, path_buffer_size: usize) -> RacResult<UrlComponents> {
    unsafe {
        let url_wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
        let mut components: URL_COMPONENTS = std::mem::zeroed();
        components.dwStructSize = size_of::<URL_COMPONENTS>() as u32;

        let mut host_buffer = vec![0u16; 256];
        let mut path_buffer = vec![0u16; path_buffer_size];

        components.lpszHostName = windows::core::PWSTR(host_buffer.as_mut_ptr());
        components.dwHostNameLength = host_buffer.len() as u32;
        components.lpszUrlPath = windows::core::PWSTR(path_buffer.as_mut_ptr());
        components.dwUrlPathLength = path_buffer.len() as u32;

        WinHttpCrackUrl(&url_wide, 0, &mut components)
            .map_err(|_| RacError::UpdateError("Failed to parse URL".into()))?;

        let host = String::from_utf16_lossy(&host_buffer[..components.dwHostNameLength as usize]);
        let path = String::from_utf16_lossy(&path_buffer[..components.dwUrlPathLength as usize]);

        Ok(UrlComponents {
            host,
            path,
            port: components.nPort,
        })
    }
}

pub fn query_status_code(request: &WinHttpHandle) -> RacResult<u32> {
    unsafe {
        let mut status_code: u32 = 0;
        let mut size = size_of::<u32>() as u32;

        WinHttpQueryHeaders(
            request.as_ptr(),
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

pub fn query_content_length(request: &WinHttpHandle) -> u64 {
    unsafe {
        let mut content_length: u64 = 0;
        let mut size = size_of::<u64>() as u32;

        let _ = WinHttpQueryHeaders(
            request.as_ptr(),
            WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut content_length as *mut u64 as *mut _),
            &mut size,
            std::ptr::null_mut(),
        );

        content_length
    }
}

pub fn open_session(user_agent: &str) -> RacResult<WinHttpHandle> {
    unsafe {
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

        let http2: u32 = WINHTTP_PROTOCOL_FLAG_HTTP2;
        let _ = WinHttpSetOption(
            Some(session.as_ptr() as *const _),
            WINHTTP_OPTION_ENABLE_HTTP_PROTOCOL,
            Some(&http2.to_ne_bytes()),
        );

        Ok(session)
    }
}

pub fn connect(session: &WinHttpHandle, host: &str, port: u16) -> RacResult<WinHttpHandle> {
    unsafe {
        WinHttpHandle::new(WinHttpConnect(
            session.as_ptr(),
            &HSTRING::from(host),
            port,
            0,
        ))
        .ok_or_else(|| {
            let error_code = windows::Win32::Foundation::GetLastError().0;
            RacError::UpdateError(format!(
                "Failed to connect to {} (error code: 0x{:X})",
                host, error_code
            ))
        })
    }
}

pub fn open_request(connect: &WinHttpHandle, path: &str) -> RacResult<WinHttpHandle> {
    unsafe {
        WinHttpHandle::new(WinHttpOpenRequest(
            connect.as_ptr(),
            &HSTRING::from("GET"),
            &HSTRING::from(path),
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
        })
    }
}

pub fn configure_request(request: &WinHttpHandle, user_agent: &str) -> RacResult<()> {
    unsafe {
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

        let revocation_flag: u32 = WINHTTP_ENABLE_SSL_REVOCATION;
        let result = WinHttpSetOption(
            Some(request.as_ptr() as *const _),
            WINHTTP_OPTION_ENABLE_FEATURE,
            Some(&revocation_flag.to_ne_bytes()),
        );

        if result.is_err() {
            eprintln!("Failed to enable SSL revocation checking");
        }

        let headers = HSTRING::from(format!("User-Agent: {}\r\n", user_agent));
        let _ = WinHttpAddRequestHeaders(request.as_ptr(), &headers, WINHTTP_ADDREQ_FLAG_ADD);

        Ok(())
    }
}

pub fn send_request(request: &WinHttpHandle) -> RacResult<()> {
    unsafe {
        WinHttpSendRequest(request.as_ptr(), None, None, 0, 0, 0).map_err(|e| {
            let error_code = windows::Win32::Foundation::GetLastError().0;
            if error_code == ERROR_WINHTTP_SECURE_CERT_REV_FAILED {
                return RacError::UpdateError(
                    "Certificate revocation check failed. The certificate may have been revoked or the CRL/OCSP server is unreachable.".to_string()
                );
            }

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

        Ok(())
    }
}

pub fn handle_status_code(status_code: u32, context: &str) -> RacResult<()> {
    match status_code {
        200 => Ok(()),
        301 | 302 | 303 | 307 | 308 => Err(RacError::UpdateError(format!(
            "Redirect not handled properly ({}). This shouldn't happen with auto-redirect enabled.",
            status_code
        ))),
        304 => Err(RacError::UpdateError("Not modified (304)".to_string())),
        403 => Err(RacError::UpdateError(format!(
            "{} forbidden (403). Access denied or rate limited.",
            context
        ))),
        404 => Err(RacError::UpdateError(format!(
            "{} not found (404). URL may be invalid.",
            context
        ))),
        429 => Err(RacError::UpdateError(
            "Rate limited (429). Too many requests. Try again later.".to_string(),
        )),
        500..=599 => Err(RacError::UpdateError(format!(
            "Server error ({}). Service temporarily unavailable.",
            status_code
        ))),
        _ => Err(RacError::UpdateError(format!(
            "HTTP error: {}",
            status_code
        ))),
    }
}

pub fn read_response_body(request: &WinHttpHandle) -> RacResult<Vec<u8>> {
    let mut response_data = Vec::new();
    let mut buffer = vec![0u8; 8192];

    loop {
        let mut bytes_read: u32 = 0;
        let result = unsafe {
            WinHttpReadData(
                request.as_ptr(),
                buffer.as_mut_ptr() as *mut _,
                buffer.len() as u32,
                &mut bytes_read,
            )
        };

        if result.is_err() || bytes_read == 0 {
            break;
        }

        if response_data.len().saturating_add(bytes_read as usize) > MAX_RESPONSE_SIZE {
            return Err(RacError::UpdateError(format!(
                "Response too large (exceeds {} MB limit)",
                MAX_RESPONSE_SIZE / (1024 * 1024)
            )));
        }

        response_data.extend_from_slice(&buffer[..bytes_read as usize]);
    }

    Ok(response_data)
}
