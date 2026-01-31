use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ResourceError {
    SdkNotFound {
        searched_locations: Vec<PathBuf>,
    },
    ResourceCompilerNotFound {
        sdk_path: PathBuf,
        expected_path: PathBuf,
    },
    SdkHeadersNotFound {
        sdk_path: PathBuf,
        missing_paths: Vec<PathBuf>,
    },
    ResourceFileNotFound {
        path: PathBuf,
    },
    ReferencedFileNotFound {
        rc_path: PathBuf,
        referenced_path: PathBuf,
    },
    CompilationFailed {
        exit_code: i32,
        stdout: String,
        stderr: String,
        command: String,
    },
    OutputDirectoryCreation {
        path: PathBuf,
        source: io::Error,
    },
    IoError {
        operation: String,
        source: io::Error,
    },
    ManifestValidation {
        path: PathBuf,
        reason: String,
    },
    UnsupportedEnvironment {
        reason: String,
    },
    SdkVersionNotFound {
        sdk_path: PathBuf,
    },
    ArchitectureNotSupported {
        sdk_path: PathBuf,
        architecture: String,
    },
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::SdkNotFound { searched_locations } => {
                writeln!(f, "Windows SDK not found")?;
                writeln!(f)?;
                writeln!(f, "Searched locations:")?;
                for loc in searched_locations {
                    writeln!(f, "  - {}", loc.display())?;
                }
                writeln!(f)?;
                writeln!(f, "To resolve this issue:")?;
                writeln!(f, "  1. Install the Windows SDK from:")?;
                writeln!(
                    f,
                    "     https://developer.microsoft.com/windows/downloads/windows-sdk/"
                )?;
                writeln!(
                    f,
                    "  2. Or install Visual Studio with the \"Desktop development with C++\" workload"
                )?;
                writeln!(f)?;
                write!(
                    f,
                    "If the SDK is installed in a non-standard location, set the "
                )?;
                write!(f, "WindowsSdkDir environment variable to the SDK root.")
            }

            ResourceError::ResourceCompilerNotFound {
                sdk_path,
                expected_path,
            } => {
                writeln!(f, "Resource compiler (rc.exe) not found")?;
                writeln!(f)?;
                writeln!(f, "SDK path: {}", sdk_path.display())?;
                writeln!(f, "Expected rc.exe at: {}", expected_path.display())?;
                writeln!(f)?;
                writeln!(f, "The Windows SDK installation may be incomplete.")?;
                write!(
                    f,
                    "Try reinstalling the Windows SDK or Visual Studio Build Tools."
                )
            }

            ResourceError::SdkHeadersNotFound {
                sdk_path,
                missing_paths,
            } => {
                writeln!(f, "Windows SDK headers not found")?;
                writeln!(f)?;
                writeln!(f, "SDK path: {}", sdk_path.display())?;
                writeln!(f, "Missing header directories:")?;
                for path in missing_paths {
                    writeln!(f, "  - {}", path.display())?;
                }
                writeln!(f)?;
                write!(
                    f,
                    "The Windows SDK installation may be incomplete or corrupted."
                )
            }

            ResourceError::ResourceFileNotFound { path } => {
                writeln!(f, "Resource file not found: {}", path.display())?;
                writeln!(f)?;
                write!(
                    f,
                    "Ensure the .rc file exists and the path is correct relative to Cargo.toml."
                )
            }

            ResourceError::ReferencedFileNotFound {
                rc_path,
                referenced_path,
            } => {
                writeln!(f, "File referenced in resource script not found")?;
                writeln!(f)?;
                writeln!(f, "Resource file: {}", rc_path.display())?;
                writeln!(f, "Missing reference: {}", referenced_path.display())?;
                writeln!(f)?;
                write!(f, "Ensure all files referenced in the .rc file exist.")
            }

            ResourceError::CompilationFailed {
                exit_code,
                stdout,
                stderr,
                command,
            } => {
                writeln!(f, "Resource compilation failed (exit code: {})", exit_code)?;
                writeln!(f)?;
                writeln!(f, "Command: {}", command)?;

                if !stdout.is_empty() {
                    writeln!(f)?;
                    writeln!(f, "Output:")?;
                    for line in stdout.lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }

                if !stderr.is_empty() {
                    writeln!(f)?;
                    writeln!(f, "Error output:")?;
                    for line in stderr.lines() {
                        writeln!(f, "  {}", line)?;
                    }
                }

                writeln!(f)?;
                write!(
                    f,
                    "Tip: Check the resource file for syntax errors and verify all referenced files exist."
                )
            }

            ResourceError::OutputDirectoryCreation { path, source } => {
                writeln!(f, "Failed to create output directory")?;
                writeln!(f)?;
                writeln!(f, "Path: {}", path.display())?;
                write!(f, "Error: {}", source)
            }

            ResourceError::IoError { operation, source } => {
                writeln!(f, "I/O error during {}", operation)?;
                writeln!(f)?;
                write!(f, "Error: {}", source)
            }

            ResourceError::ManifestValidation { path, reason } => {
                writeln!(f, "Manifest validation failed")?;
                writeln!(f)?;
                writeln!(f, "Resource file: {}", path.display())?;
                writeln!(f, "Reason: {}", reason)?;
                writeln!(f)?;
                writeln!(f, "Ensure your .rc file includes a manifest resource like:")?;
                write!(
                    f,
                    "  CREATEPROCESS_MANIFEST_RESOURCE_ID RT_MANIFEST \"Manifest.xml\""
                )
            }

            ResourceError::UnsupportedEnvironment { reason } => {
                writeln!(f, "Unsupported build environment")?;
                writeln!(f)?;
                write!(f, "Reason: {}", reason)
            }

            ResourceError::SdkVersionNotFound { sdk_path } => {
                writeln!(f, "Could not determine Windows SDK version")?;
                writeln!(f)?;
                writeln!(f, "SDK path: {}", sdk_path.display())?;
                writeln!(f)?;
                write!(f, "No version directories found in the SDK Include folder.")
            }

            ResourceError::ArchitectureNotSupported {
                sdk_path,
                architecture,
            } => {
                writeln!(f, "SDK binaries for {} not found", architecture)?;
                writeln!(f)?;
                writeln!(f, "SDK path: {}", sdk_path.display())?;
                writeln!(f)?;
                write!(
                    f,
                    "The SDK installation may not include {} binaries.",
                    architecture
                )
            }
        }
    }
}

impl std::error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ResourceError::OutputDirectoryCreation { source, .. } => Some(source),
            ResourceError::IoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for ResourceError {
    fn from(error: io::Error) -> Self {
        ResourceError::IoError {
            operation: "file operation".to_string(),
            source: error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_not_found_display() {
        let error = ResourceError::SdkNotFound {
            searched_locations: vec![
                PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10"),
                PathBuf::from(r"C:\Program Files\Windows Kits\10"),
            ],
        };

        let message = error.to_string();
        assert!(message.contains("Windows SDK not found"));
        assert!(message.contains("Searched locations:"));
        assert!(message.contains("Windows Kits"));
    }

    #[test]
    fn test_compilation_failed_display() {
        let error = ResourceError::CompilationFailed {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error RC2135 : file not found: icon.ico".to_string(),
            command: "rc.exe /nologo test.rc".to_string(),
        };

        let message = error.to_string();
        assert!(message.contains("exit code: 1"));
        assert!(message.contains("RC2135"));
        assert!(message.contains("icon.ico"));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ResourceError>();
        assert_sync::<ResourceError>();
    }
}
