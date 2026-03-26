use crate::native::InstalledHostedAddonProcess;
use crate::platform::{HostedAddonRequest, HostedAddonResponse};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

#[allow(dead_code)]
pub(crate) struct HostedAddonProcessSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

#[allow(dead_code)]
impl HostedAddonProcessSession {
    pub(crate) fn spawn(
        process: &InstalledHostedAddonProcess,
        init: &HostedAddonRequest,
    ) -> Result<(Self, HostedAddonResponse), String> {
        let mut child = Command::new(&process.executable_path)
            .current_dir(&process.bundle_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                format!(
                    "Failed to launch hosted addon '{}' from '{}': {error}",
                    process.addon_id,
                    process.executable_path.display()
                )
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Hosted addon process stdin was not available.".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Hosted addon process stdout was not available.".to_string())?;
        let mut session = Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };
        let response = session.request(init)?;
        Ok((session, response))
    }

    pub(crate) fn request(
        &mut self,
        request: &HostedAddonRequest,
    ) -> Result<HostedAddonResponse, String> {
        let encoded = serde_json::to_string(request)
            .map_err(|error| format!("Failed to encode hosted addon request: {error}"))?;
        self.stdin
            .write_all(encoded.as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|error| format!("Failed to write hosted addon request: {error}"))?;
        self.read_response()
    }

    pub(crate) fn shutdown(mut self) -> Result<(), String> {
        let _ = self.request(&HostedAddonRequest::Shutdown);
        let _ = self.child.wait();
        Ok(())
    }

    fn read_response(&mut self) -> Result<HostedAddonResponse, String> {
        let mut line = String::new();
        let read = self
            .stdout
            .read_line(&mut line)
            .map_err(|error| format!("Failed to read hosted addon response: {error}"))?;
        if read == 0 {
            return Err("Hosted addon process closed without sending a response.".to_string());
        }
        serde_json::from_str(line.trim_end())
            .map_err(|error| format!("Failed to parse hosted addon response: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::HostedAddonProcessSession;
    use crate::native::InstalledHostedAddonProcess;
    use crate::platform::{
        AddonId, HostedAddonFrame, HostedAddonInitRequest, HostedAddonProtocol,
        HostedAddonRequest, HostedAddonResponse, HostedAddonSize, HostedAddonSurface,
        HostedAddonUpdateRequest,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    #[test]
    fn hosted_addon_process_session_round_trips_stdio_protocol() {
        let dir = temp_dir("hosted_addon_process_session_round_trips_stdio_protocol");
        let executable = dir.join("mock-addon.sh");
        fs::write(
            &executable,
            r#"#!/bin/sh
IFS= read -r init
printf '%s\n' '{"type":"ready","title":"Mock Addon","frame":{"size":{"width":320.0,"height":200.0},"commands":[],"status_line":"ready"}}'
IFS= read -r update
printf '%s\n' '{"type":"frame","frame":{"size":{"width":320.0,"height":200.0},"commands":[],"status_line":"updated"}}'
IFS= read -r shutdown
exit 0
"#,
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&executable).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&executable, perms).unwrap();

        let process = InstalledHostedAddonProcess {
            addon_id: AddonId::from("games.mock-addon"),
            protocol: HostedAddonProtocol::ShellSurfaceV1,
            executable_path: executable,
            bundle_dir: dir.clone(),
        };
        let init = HostedAddonRequest::Initialize(HostedAddonInitRequest {
            addon_id: "games.mock-addon".to_string(),
            surface: HostedAddonSurface::Desktop,
            size: HostedAddonSize {
                width: 320.0,
                height: 200.0,
            },
            scale_factor: 1.0,
        });

        let (mut session, ready) = HostedAddonProcessSession::spawn(&process, &init).unwrap();
        assert_eq!(
            ready,
            HostedAddonResponse::Ready {
                title: "Mock Addon".to_string(),
                frame: HostedAddonFrame {
                    size: HostedAddonSize {
                        width: 320.0,
                        height: 200.0,
                    },
                    clear: None,
                    commands: Vec::new(),
                    status_line: Some("ready".to_string()),
                },
            }
        );

        let frame = session
            .request(&HostedAddonRequest::Update(HostedAddonUpdateRequest {
                size: HostedAddonSize {
                    width: 320.0,
                    height: 200.0,
                },
                delta_seconds: 1.0 / 60.0,
                input: Vec::new(),
            }))
            .unwrap();
        assert_eq!(
            frame,
            HostedAddonResponse::Frame {
                frame: HostedAddonFrame {
                    size: HostedAddonSize {
                        width: 320.0,
                        height: 200.0,
                    },
                    clear: None,
                    commands: Vec::new(),
                    status_line: Some("updated".to_string()),
                },
            }
        );

        session.shutdown().unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("robcos-hosted-addon-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
