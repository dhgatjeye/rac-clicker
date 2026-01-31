use crate::error::ResourceError;
use crate::toolchain::{WindowsSdk, find_windows_sdk};
use crate::types;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone)]
pub struct ResourceCompiler {
    rc_file: PathBuf,
    defines: Vec<(String, String)>,
    include_paths: Vec<PathBuf>,
    output_name: Option<String>,
    require_manifest: bool,
}

impl ResourceCompiler {
    pub fn new(rc_file: impl AsRef<Path>) -> Self {
        Self {
            rc_file: rc_file.as_ref().to_path_buf(),
            defines: Vec::new(),
            include_paths: Vec::new(),
            output_name: None,
            require_manifest: false,
        }
    }

    pub fn define(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.defines.push((name.into(), value.into()));
        self
    }

    pub fn include(mut self, path: impl AsRef<Path>) -> Self {
        self.include_paths.push(path.as_ref().to_path_buf());
        self
    }

    pub fn output(mut self, name: impl Into<String>) -> Self {
        self.output_name = Some(name.into());
        self
    }

    pub fn manifest_required(mut self) -> Self {
        self.require_manifest = true;
        self
    }

    pub fn compile(self) -> Result<(), ResourceError> {
        if !types::is_msvc_windows_target() {
            return Ok(());
        }

        let sdk = find_windows_sdk()?;

        let manifest_dir =
            types::get_manifest_dir().ok_or_else(|| ResourceError::UnsupportedEnvironment {
                reason: "CARGO_MANIFEST_DIR not set".to_string(),
            })?;

        let out_dir =
            types::get_out_dir().ok_or_else(|| ResourceError::UnsupportedEnvironment {
                reason: "OUT_DIR not set".to_string(),
            })?;

        let rc_path = if self.rc_file.is_absolute() {
            self.rc_file.clone()
        } else {
            manifest_dir.join(&self.rc_file)
        };

        if !rc_path.exists() {
            return Err(ResourceError::ResourceFileNotFound { path: rc_path });
        }

        if self.require_manifest {
            self.validate_manifest(&rc_path)?;
        }

        let output_name = self.output_name.clone().unwrap_or_else(|| {
            rc_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "resources".to_string())
        });

        let output_path = out_dir.join(format!("{}.res", output_name));

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ResourceError::OutputDirectoryCreation {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let (command, output) = self.execute_rc(&sdk, &rc_path, &output_path)?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            return Err(ResourceError::CompilationFailed {
                exit_code,
                stdout,
                stderr,
                command,
            });
        }

        println!("cargo:rustc-link-arg-bins={}", output_path.display());

        Ok(())
    }

    fn validate_manifest(&self, rc_path: &Path) -> Result<(), ResourceError> {
        let content = fs::read_to_string(rc_path).map_err(|e| ResourceError::IoError {
            operation: format!("reading {}", rc_path.display()),
            source: e,
        })?;

        let has_manifest = content.contains("RT_MANIFEST")
            || content.contains("MANIFEST")
            || content.contains("manifest");

        if !has_manifest {
            return Err(ResourceError::ManifestValidation {
                path: rc_path.to_path_buf(),
                reason: "No manifest resource definition found in the resource file".to_string(),
            });
        }

        Ok(())
    }

    fn execute_rc(
        &self,
        sdk: &WindowsSdk,
        rc_path: &Path,
        output_path: &Path,
    ) -> Result<(String, Output), ResourceError> {
        let mut cmd = Command::new(&sdk.rc_exe);

        cmd.arg("/nologo");

        cmd.arg("/fo");
        cmd.arg(output_path);

        for include_path in &sdk.include_paths {
            cmd.arg("/I");
            cmd.arg(include_path);
        }

        let manifest_dir = types::get_manifest_dir();
        for include_path in &self.include_paths {
            let full_path = if include_path.is_absolute() {
                include_path.clone()
            } else if let Some(ref manifest) = manifest_dir {
                manifest.join(include_path)
            } else {
                include_path.clone()
            };
            cmd.arg("/I");
            cmd.arg(&full_path);
        }

        if let Some(rc_dir) = rc_path.parent() {
            cmd.arg("/I");
            cmd.arg(rc_dir);
        }

        for (name, value) in &self.defines {
            if value.is_empty() {
                cmd.arg(format!("/d{}", name));
            } else {
                cmd.arg(format!("/d{}={}", name, value));
            }
        }

        if types::is_debug_build() {
            cmd.arg("/dDEBUG");
        }

        cmd.arg(rc_path);

        if let Some(rc_dir) = rc_path.parent() {
            cmd.current_dir(rc_dir);
        }

        let command_str = format_command(&cmd);

        let output = cmd.output().map_err(|e| ResourceError::IoError {
            operation: format!("executing {}", sdk.rc_exe.display()),
            source: e,
        })?;

        Ok((command_str, output))
    }
}

fn format_command(cmd: &Command) -> String {
    let mut parts = vec![format!("{:?}", cmd.get_program())];

    for arg in cmd.get_args() {
        parts.push(format!("{:?}", arg));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_compiler_builder() {
        let compiler = ResourceCompiler::new("test.rc")
            .define("VERSION", "1.0.0")
            .define("DEBUG", "")
            .include("resources")
            .output("custom_output")
            .manifest_required();

        assert_eq!(compiler.rc_file, PathBuf::from("test.rc"));
        assert_eq!(compiler.defines.len(), 2);
        assert_eq!(
            compiler.defines[0],
            ("VERSION".to_string(), "1.0.0".to_string())
        );
        assert_eq!(compiler.defines[1], ("DEBUG".to_string(), "".to_string()));
        assert_eq!(compiler.include_paths.len(), 1);
        assert_eq!(compiler.output_name, Some("custom_output".to_string()));
        assert!(compiler.require_manifest);
    }

    #[test]
    fn test_format_command() {
        let mut cmd = Command::new("rc.exe");
        cmd.arg("/nologo").arg("/fo").arg("output.res");

        let formatted = format_command(&cmd);
        assert!(formatted.contains("rc.exe"));
        assert!(formatted.contains("/nologo"));
    }
}
