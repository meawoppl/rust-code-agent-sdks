//! Builder for launching the Codex app-server process.
//!
//! The [`AppServerBuilder`] configures and spawns `codex app-server --listen stdio://`,
//! a long-lived process that speaks JSON-RPC over newline-delimited stdio.

use log::debug;
use std::path::PathBuf;
use std::process::Stdio;

/// Builder for launching a Codex app-server process.
///
/// Produces commands of the form: `codex app-server --listen stdio://`
///
/// All model, sandbox, and approval configuration is done via JSON-RPC
/// requests after connecting, not via CLI flags.
#[derive(Debug, Clone)]
pub struct AppServerBuilder {
    command: PathBuf,
    working_directory: Option<PathBuf>,
}

impl Default for AppServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AppServerBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            command: PathBuf::from("codex"),
            working_directory: None,
        }
    }

    /// Set custom path to the codex binary.
    pub fn command<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.command = path.into();
        self
    }

    /// Set the working directory for the app-server process.
    pub fn working_directory<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Resolve the command path, using `which` for non-absolute paths.
    fn resolve_command(&self) -> crate::error::Result<PathBuf> {
        if self.command.is_absolute() {
            return Ok(self.command.clone());
        }
        which::which(&self.command).map_err(|_| crate::error::Error::BinaryNotFound {
            name: self.command.display().to_string(),
        })
    }

    /// Build the command arguments.
    fn build_args(&self) -> Vec<String> {
        vec![
            "app-server".to_string(),
            "--listen".to_string(),
            "stdio://".to_string(),
        ]
    }

    /// Spawn the app-server process asynchronously.
    #[cfg(feature = "async-client")]
    pub async fn spawn(self) -> crate::error::Result<tokio::process::Child> {
        let resolved = self.resolve_command()?;
        let args = self.build_args();

        debug!(
            "[CLI] Spawning async app-server: {} {}",
            resolved.display(),
            args.join(" ")
        );

        let mut cmd = tokio::process::Command::new(&resolved);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.spawn().map_err(crate::error::Error::Io)
    }

    /// Spawn the app-server process synchronously.
    pub fn spawn_sync(self) -> crate::error::Result<std::process::Child> {
        let resolved = self.resolve_command()?;
        let args = self.build_args();

        debug!(
            "[CLI] Spawning sync app-server: {} {}",
            resolved.display(),
            args.join(" ")
        );

        let mut cmd = std::process::Command::new(&resolved);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(ref dir) = self.working_directory {
            cmd.current_dir(dir);
        }

        cmd.spawn().map_err(crate::error::Error::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let builder = AppServerBuilder::new();
        let args = builder.build_args();

        assert_eq!(args, vec!["app-server", "--listen", "stdio://"]);
    }

    #[test]
    fn test_custom_command() {
        let builder = AppServerBuilder::new().command("/usr/local/bin/codex");
        assert_eq!(builder.command, PathBuf::from("/usr/local/bin/codex"));
    }

    #[test]
    fn test_working_directory() {
        let builder = AppServerBuilder::new().working_directory("/tmp/work");
        assert_eq!(builder.working_directory, Some(PathBuf::from("/tmp/work")));
    }
}
