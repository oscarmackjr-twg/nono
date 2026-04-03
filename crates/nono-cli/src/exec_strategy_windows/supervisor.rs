use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsSupervisorLifecycleState {
    Initializing,
    ControlChannelReady,
    LaunchingChild,
    WaitingForChild,
    ShuttingDown,
    Completed,
}

impl WindowsSupervisorLifecycleState {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Initializing => "initializing",
            Self::ControlChannelReady => "control-channel-ready",
            Self::LaunchingChild => "launching-child",
            Self::WaitingForChild => "waiting-for-child",
            Self::ShuttingDown => "shutting-down",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug)]
pub(super) enum WindowsSupervisedChild {
    Native {
        process: OwnedHandle,
        _thread: OwnedHandle,
    },
}

impl WindowsSupervisedChild {
    pub(super) fn poll_exit_code(&mut self) -> Result<Option<i32>> {
        match self {
            Self::Native { process, .. } => {
                let wait_result = unsafe {
                    // SAFETY: `process.0` is a valid process handle owned by this child wrapper.
                    WaitForSingleObject(process.0, 0)
                };
                if wait_result == 0 {
                    let mut exit_code = 0u32;
                    let ok = unsafe {
                        // SAFETY: `process.0` remains a valid process handle for the duration
                        // of this query and `exit_code` points to writable memory.
                        GetExitCodeProcess(process.0, &mut exit_code)
                    };
                    if ok == 0 {
                        return Err(NonoError::SandboxInit(
                            "Failed to query Windows supervised child exit code".to_string(),
                        ));
                    }
                    Ok(Some(exit_code as i32))
                } else if wait_result == 0x0000_0102 {
                    Ok(None)
                } else {
                    Err(NonoError::SandboxInit(format!(
                        "Windows supervisor failed while waiting for child process state: {}",
                        std::io::Error::last_os_error()
                    )))
                }
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct WindowsSupervisorRuntime {
    session_id: String,
    requested_features: Vec<String>,
    transport_name: String,
    _parent_control: nono::SupervisorSocket,
    child_control: Option<nono::SupervisorSocket>,
    started_at: Instant,
    pub(super) state: WindowsSupervisorLifecycleState,
    audit_log: Vec<AuditEntry>,
}

impl WindowsSupervisorRuntime {
    pub(super) fn initialize(supervisor: &SupervisorConfig<'_>) -> Result<Self> {
        let started_at = Instant::now();
        let (parent_control, child_control) = initialize_supervisor_control_channel()?;
        let transport_name = parent_control.transport_name().to_string();
        let mut runtime = Self {
            session_id: supervisor.session_id.to_string(),
            requested_features: supervisor
                .requested_features
                .iter()
                .map(|feature| (*feature).to_string())
                .collect(),
            transport_name,
            _parent_control: parent_control,
            child_control: Some(child_control),
            started_at,
            state: WindowsSupervisorLifecycleState::Initializing,
            audit_log: Vec::new(),
        };
        runtime.state = WindowsSupervisorLifecycleState::ControlChannelReady;
        Ok(runtime)
    }

    pub(super) fn transport_name(&self) -> &str {
        self.transport_name.as_str()
    }

    pub(super) fn run_child_event_loop(
        &mut self,
        child: &mut WindowsSupervisedChild,
    ) -> Result<i32> {
        self.state = WindowsSupervisorLifecycleState::WaitingForChild;
        tracing::debug!(
            "Windows supervisor event loop entering wait phase (session: {}, transport: {}, state: {}, features: {})",
            self.session_id,
            self.transport_name,
            self.state.label(),
            if self.requested_features.is_empty() {
                "none".to_string()
            } else {
                self.requested_features.join(", ")
            }
        );

        loop {
            if let Some(exit_code) = child.poll_exit_code()? {
                self.state = WindowsSupervisorLifecycleState::ShuttingDown;
                self.shutdown();
                self.state = WindowsSupervisorLifecycleState::Completed;
                tracing::debug!(
                    "Windows supervisor event loop completed (session: {}, transport: {}, exit_code: {}, elapsed_ms: {})",
                    self.session_id,
                    self.transport_name,
                    exit_code,
                    self.started_at.elapsed().as_millis()
                );
                return Ok(exit_code);
            }

            std::thread::sleep(WINDOWS_SUPERVISOR_POLL_INTERVAL);
        }
    }

    pub(super) fn startup_failure(&mut self, message: String) -> NonoError {
        self.shutdown();
        NonoError::SandboxInit(format!(
            "Windows supervised execution failed during {} (session: {}, transport: {}, supervisor_audit_entries: {}): {}",
            self.state.label(),
            self.session_id,
            self.transport_name,
            self.audit_log.len(),
            message
        ))
    }

    pub(super) fn command_failure(&mut self, message: String) -> NonoError {
        self.shutdown();
        NonoError::CommandExecution(std::io::Error::other(format!(
            "Windows supervised execution failed during {} (session: {}, transport: {}, supervisor_audit_entries: {}): {}",
            self.state.label(),
            self.session_id,
            self.transport_name,
            self.audit_log.len(),
            message
        )))
    }

    pub(super) fn shutdown(&mut self) {
        let _ = self.child_control.take();
        self.state = WindowsSupervisorLifecycleState::ShuttingDown;
    }
}

impl Drop for WindowsSupervisorRuntime {
    fn drop(&mut self) {
        if self.state != WindowsSupervisorLifecycleState::Completed {
            self.shutdown();
        }
    }
}

pub(super) fn initialize_supervisor_control_channel(
) -> Result<(nono::SupervisorSocket, nono::SupervisorSocket)> {
    nono::SupervisorSocket::pair().map_err(|e| {
        NonoError::SandboxInit(format!(
            "Failed to initialize Windows supervisor control channel: {e}"
        ))
    })
}

#[cfg(test)]
pub(super) fn open_windows_supervisor_path(
    path: &Path,
    access: &nono::AccessMode,
) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    match access {
        nono::AccessMode::Read => {
            options.read(true);
        }
        nono::AccessMode::Write => {
            options.write(true);
        }
        nono::AccessMode::ReadWrite => {
            options.read(true).write(true);
        }
    }

    options.open(path).map_err(|e| {
        NonoError::SandboxInit(format!(
            "Windows supervisor failed to open approved path {}: {}",
            path.display(),
            e
        ))
    })
}

#[cfg(test)]
pub(super) fn handle_windows_supervisor_message(
    sock: &mut nono::SupervisorSocket,
    msg: nono::supervisor::SupervisorMessage,
    approval_backend: &dyn ApprovalBackend,
    target_process: nono::BrokerTargetProcess,
    seen_request_ids: &mut HashSet<String>,
    audit_log: &mut Vec<AuditEntry>,
) -> Result<()> {
    match msg {
        nono::supervisor::SupervisorMessage::Request(request) => {
            let started_at = Instant::now();
            if seen_request_ids.contains(&request.request_id) {
                let decision = nono::ApprovalDecision::Denied {
                    reason: "Duplicate request_id rejected (replay detected)".to_string(),
                };
                audit_log.push(AuditEntry {
                    timestamp: SystemTime::now(),
                    request: request.clone(),
                    decision: decision.clone(),
                    backend: approval_backend.backend_name().to_string(),
                    duration_ms: started_at.elapsed().as_millis() as u64,
                });
                return sock.send_response(&nono::supervisor::SupervisorResponse::Decision {
                    request_id: request.request_id,
                    decision,
                    grant: None,
                });
            }
            seen_request_ids.insert(request.request_id.clone());

            let decision = approval_backend
                .request_capability(&request)
                .unwrap_or_else(|e| nono::ApprovalDecision::Denied {
                    reason: format!("Approval backend error: {e}"),
                });

            let grant = if decision.is_granted() {
                let file = open_windows_supervisor_path(&request.path, &request.access)?;
                Some(nono::supervisor::socket::broker_file_handle_to_process(
                    &file,
                    target_process,
                    request.access,
                )?)
            } else {
                None
            };

            audit_log.push(AuditEntry {
                timestamp: SystemTime::now(),
                request: request.clone(),
                decision: decision.clone(),
                backend: approval_backend.backend_name().to_string(),
                duration_ms: started_at.elapsed().as_millis() as u64,
            });

            sock.send_response(&nono::supervisor::SupervisorResponse::Decision {
                request_id: request.request_id,
                decision,
                grant,
            })
        }
        nono::supervisor::SupervisorMessage::OpenUrl(url_request) => sock
            .send_response(&nono::supervisor::SupervisorResponse::UrlOpened {
            request_id: url_request.request_id,
            success: false,
            error: Some(
                "Windows delegated browser-open flows are not available yet. Windows supervised child processes do not have an attached supervisor control channel for open-url requests."
                    .to_string(),
            ),
        }),
    }
}
