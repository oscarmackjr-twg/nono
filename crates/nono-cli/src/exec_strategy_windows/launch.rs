use super::*;

impl OwnedHandle {
    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                // SAFETY: This handle is owned by the wrapper and is closed
                // exactly once on drop.
                CloseHandle(self.0);
            }
        }
    }
}

impl Drop for ProcessContainment {
    fn drop(&mut self) {
        if !self.job.is_null() {
            unsafe {
                // SAFETY: `self.job` was returned by CreateJobObjectW and is
                // owned by this struct. Closing the handle releases the job.
                CloseHandle(self.job);
            }
        }
    }
}

pub(super) fn create_process_containment() -> Result<ProcessContainment> {
    let job = unsafe {
        // SAFETY: Null security attributes and name are valid for creating an
        // unnamed job object owned by the current process.
        CreateJobObjectW(std::ptr::null(), std::ptr::null())
    };
    if job.is_null() {
        return Err(NonoError::SandboxInit(
            "Failed to create Windows process containment job object".to_string(),
        ));
    }

    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe {
        // SAFETY: JOBOBJECT_EXTENDED_LIMIT_INFORMATION is a plain Win32 FFI
        // struct. Zero-initialization is the standard baseline before setting
        // the specific fields we rely on below.
        std::mem::zeroed()
    };
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;

    let ok = unsafe {
        // SAFETY: `limits` points to initialized memory of the exact struct
        // type required for JobObjectExtendedLimitInformation.
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        unsafe {
            // SAFETY: `job` is an owned handle created above.
            CloseHandle(job);
        }
        return Err(NonoError::SandboxInit(
            "Failed to configure Windows process containment job object".to_string(),
        ));
    }

    Ok(ProcessContainment { job })
}

pub(super) fn apply_process_handle_to_containment(
    containment: &ProcessContainment,
    process: HANDLE,
) -> Result<()> {
    let ok = unsafe {
        // SAFETY: `containment.job` is a live job handle owned by the current
        // process, and `process` is a live process handle returned by
        // CreateProcessW/CreateProcessAsUserW.
        AssignProcessToJobObject(containment.job, process)
    };
    if ok == 0 {
        return Err(NonoError::SandboxInit(
            "Failed to assign Windows child process to process containment job object".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn terminate_suspended_process(process: HANDLE, reason: &str) {
    let _ = unsafe {
        // SAFETY: `process` is a live process handle that the caller owns for the
        // duration of this cleanup path. Best-effort termination preserves fail-closed behavior.
        TerminateProcess(process, 1)
    };
    tracing::debug!("terminated suspended Windows child after containment failure: {reason}");
}

pub(super) fn resume_contained_process(process: HANDLE, thread: HANDLE) -> Result<()> {
    let resume_result = unsafe {
        // SAFETY: `thread` is the live primary thread handle returned by
        // CreateProcessW/CreateProcessAsUserW. Resuming it starts execution only
        // after containment has already been attached.
        ResumeThread(thread)
    };
    if resume_result == u32::MAX {
        terminate_suspended_process(process, "ResumeThread failed");
        return Err(NonoError::SandboxInit(
            "Failed to resume Windows child process after attaching containment".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn prepare_runtime_hardened_args(
    resolved_program: &Path,
    args: &[String],
) -> Vec<String> {
    let program_name = resolved_program
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match program_name.as_str() {
        "cmd.exe" | "cmd" => {
            if args
                .first()
                .is_some_and(|arg| arg.eq_ignore_ascii_case("/d"))
            {
                args.to_vec()
            } else {
                let mut hardened = Vec::with_capacity(args.len() + 1);
                hardened.push("/d".to_string());
                hardened.extend_from_slice(args);
                hardened
            }
        }
        "powershell.exe" | "powershell" | "pwsh.exe" | "pwsh" => {
            let mut hardened = Vec::with_capacity(args.len() + 3);
            let mut has_no_profile = false;
            let mut has_non_interactive = false;
            let mut has_no_logo = false;

            for arg in args {
                if arg.eq_ignore_ascii_case("-NoProfile") {
                    has_no_profile = true;
                } else if arg.eq_ignore_ascii_case("-NonInteractive") {
                    has_non_interactive = true;
                } else if arg.eq_ignore_ascii_case("-NoLogo") {
                    has_no_logo = true;
                }
            }

            if !has_no_profile {
                hardened.push("-NoProfile".to_string());
            }
            if !has_non_interactive {
                hardened.push("-NonInteractive".to_string());
            }
            if !has_no_logo {
                hardened.push("-NoLogo".to_string());
            }
            hardened.extend_from_slice(args);
            hardened
        }
        "cscript.exe" | "cscript" => {
            let mut hardened = Vec::with_capacity(args.len() + 2);
            let mut has_no_logo = false;
            let mut has_batch = false;

            for arg in args {
                if arg.eq_ignore_ascii_case("//NoLogo") {
                    has_no_logo = true;
                } else if arg.eq_ignore_ascii_case("//B") {
                    has_batch = true;
                }
            }

            if !has_no_logo {
                hardened.push("//NoLogo".to_string());
            }
            if !has_batch {
                hardened.push("//B".to_string());
            }
            hardened.extend_from_slice(args);
            hardened
        }
        "wscript.exe" | "wscript" => {
            if args.iter().any(|arg| arg.eq_ignore_ascii_case("//NoLogo")) {
                args.to_vec()
            } else {
                let mut hardened = Vec::with_capacity(args.len() + 1);
                hardened.push("//NoLogo".to_string());
                hardened.extend_from_slice(args);
                hardened
            }
        }
        _ => args.to_vec(),
    }
}

pub(super) fn build_child_env(config: &ExecConfig<'_>) -> Vec<(String, String)> {
    let mut env_pairs = Vec::new();
    for (key, value) in std::env::vars() {
        if !should_skip_env_var(
            &key,
            &config.env_vars,
            &[
                "NONO_CAP_FILE",
                "PATH",
                "PATHEXT",
                "COMSPEC",
                "SystemRoot",
                "windir",
                "SystemDrive",
                "NoDefaultCurrentDirectoryInExePath",
                "TMP",
                "TEMP",
                "TMPDIR",
                "APPDATA",
                "LOCALAPPDATA",
                "HOME",
                "USERPROFILE",
                "HOMEDRIVE",
                "HOMEPATH",
                "XDG_CONFIG_HOME",
                "XDG_CACHE_HOME",
                "XDG_DATA_HOME",
                "XDG_STATE_HOME",
                "PROGRAMDATA",
                "ALLUSERSPROFILE",
                "PUBLIC",
                "ProgramFiles",
                "ProgramFiles(x86)",
                "ProgramW6432",
                "CommonProgramFiles",
                "CommonProgramFiles(x86)",
                "CommonProgramW6432",
                "OneDrive",
                "OneDriveConsumer",
                "OneDriveCommercial",
                "INETCACHE",
                "INETCOOKIES",
                "INETHISTORY",
                "PSModulePath",
                "PSModuleAnalysisCachePath",
                "CARGO_HOME",
                "RUSTUP_HOME",
                "DOTNET_CLI_HOME",
                "NUGET_PACKAGES",
                "NUGET_HTTP_CACHE_PATH",
                "NUGET_PLUGINS_CACHE_PATH",
                "ChocolateyInstall",
                "ChocolateyToolsLocation",
                "VCPKG_ROOT",
                "NPM_CONFIG_CACHE",
                "NPM_CONFIG_USERCONFIG",
                "YARN_CACHE_FOLDER",
                "PIP_CACHE_DIR",
                "PIP_CONFIG_FILE",
                "PIP_BUILD_TRACKER",
                "PYTHONPYCACHEPREFIX",
                "PYTHONUSERBASE",
                "GOCACHE",
                "GOMODCACHE",
                "GOPATH",
                "HISTFILE",
                "LESSHISTFILE",
                "NODE_REPL_HISTORY",
                "PYTHONHISTFILE",
                "SQLITE_HISTORY",
                "IPYTHONDIR",
                "GEM_HOME",
                "GEM_PATH",
                "BUNDLE_USER_HOME",
                "BUNDLE_USER_CACHE",
                "BUNDLE_USER_CONFIG",
                "BUNDLE_APP_CONFIG",
                "COMPOSER_HOME",
                "COMPOSER_CACHE_DIR",
                "GRADLE_USER_HOME",
                "MAVEN_USER_HOME",
                "RIPGREP_CONFIG_PATH",
                "AWS_SHARED_CREDENTIALS_FILE",
                "AWS_CONFIG_FILE",
                "AZURE_CONFIG_DIR",
                "KUBECONFIG",
                "DOCKER_CONFIG",
                "CLOUDSDK_CONFIG",
                "GIT_CONFIG_GLOBAL",
                "GNUPGHOME",
                "TF_CLI_CONFIG_FILE",
                "TF_DATA_DIR",
            ],
        ) {
            env_pairs.push((key, value));
        }
    }

    if let Some(cap_file) = config.cap_file {
        env_pairs.push((
            "NONO_CAP_FILE".to_string(),
            cap_file.to_string_lossy().into_owned(),
        ));
    }

    for (key, value) in &config.env_vars {
        env_pairs.push(((*key).to_string(), (*value).to_string()));
    }

    env_pairs
}

pub(super) fn build_windows_environment_block(env_pairs: &[(String, String)]) -> Vec<u16> {
    let mut deduped = Vec::with_capacity(env_pairs.len());
    let mut seen_keys = HashSet::with_capacity(env_pairs.len());
    for (key, value) in env_pairs.iter().rev() {
        let folded = key.to_ascii_lowercase();
        if seen_keys.insert(folded) {
            deduped.push((key.clone(), value.clone()));
        }
    }
    deduped.reverse();

    let mut sorted = deduped;
    sorted.sort_by(|left, right| {
        left.0
            .to_ascii_lowercase()
            .cmp(&right.0.to_ascii_lowercase())
    });

    let mut block = Vec::new();
    for (key, value) in sorted {
        let pair = format!("{key}={value}");
        block.extend(OsStr::new(&pair).encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}

pub(super) fn quote_windows_arg(arg: &str) -> String {
    if !arg.contains([' ', '\t', '"']) && !arg.is_empty() {
        return arg.to_string();
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                backslashes = 0;
                quoted.push(ch);
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

pub(super) fn build_command_line(resolved_program: &Path, args: &[String]) -> Vec<u16> {
    let mut command_line = quote_windows_arg(&resolved_program.to_string_lossy());
    for arg in args {
        command_line.push(' ');
        command_line.push_str(&quote_windows_arg(arg));
    }
    OsStr::new(&command_line)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub(super) fn should_use_low_integrity_windows_launch(caps: &CapabilitySet) -> bool {
    let policy = Sandbox::windows_filesystem_policy(caps);
    policy.has_rules()
}

pub(super) fn create_low_integrity_primary_token() -> Result<OwnedHandle> {
    let mut current_token = std::ptr::null_mut();
    let opened = unsafe {
        // SAFETY: We pass a valid mutable out-pointer and request access on the
        // current process token only.
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_DEFAULT,
            &mut current_token,
        )
    };
    if opened == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to open Windows process token for low-integrity launch (GetLastError={})",
            unsafe { GetLastError() }
        )));
    }
    let current_token = OwnedHandle(current_token);

    let mut primary_token = std::ptr::null_mut();
    let duplicated = unsafe {
        // SAFETY: We duplicate the current process token into a primary token
        // for child process creation.
        DuplicateTokenEx(
            current_token.raw(),
            TOKEN_ASSIGN_PRIMARY | TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ADJUST_DEFAULT,
            std::ptr::null(),
            SecurityImpersonation as SECURITY_IMPERSONATION_LEVEL,
            TokenPrimary,
            &mut primary_token,
        )
    };
    if duplicated == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to duplicate Windows process token for low-integrity launch (GetLastError={})",
            unsafe { GetLastError() }
        )));
    }
    let primary_token = OwnedHandle(primary_token);

    let mut sid_buffer = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut sid_size = sid_buffer.len() as u32;
    let created = unsafe {
        // SAFETY: The destination buffer is valid and sized per
        // SECURITY_MAX_SID_SIZE for a well-known SID.
        CreateWellKnownSid(
            WinLowLabelSid,
            std::ptr::null_mut(),
            sid_buffer.as_mut_ptr() as *mut _,
            &mut sid_size,
        )
    };
    if created == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to create Windows low-integrity SID (GetLastError={})",
            unsafe { GetLastError() }
        )));
    }

    let mut label = TOKEN_MANDATORY_LABEL {
        Label: SID_AND_ATTRIBUTES {
            Sid: sid_buffer.as_mut_ptr() as *mut _,
            Attributes: SE_GROUP_INTEGRITY as u32,
        },
    };
    let label_size = size_of::<TOKEN_MANDATORY_LABEL>() + sid_size as usize;
    let adjusted = unsafe {
        // SAFETY: The token handle is valid and the TOKEN_MANDATORY_LABEL
        // points to a valid low-integrity SID buffer for the duration
        // of the call.
        SetTokenInformation(
            primary_token.raw(),
            TokenIntegrityLevel,
            &mut label as *mut _ as *mut _,
            label_size as u32,
        )
    };
    if adjusted == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to lower Windows child token integrity level (GetLastError={})",
            unsafe { GetLastError() }
        )));
    }

    Ok(primary_token)
}

pub(super) fn execute_direct_with_low_integrity(
    config: &ExecConfig<'_>,
    launch_program: &Path,
    containment: &ProcessContainment,
    cmd_args: &[String],
) -> Result<i32> {
    let mut child =
        spawn_low_integrity_windows_child(config, launch_program, containment, cmd_args)?;
    let Some(exit_code) = child.poll_exit_code()? else {
        loop {
            if let Some(exit_code) = child.poll_exit_code()? {
                return Ok(exit_code);
            }
            std::thread::sleep(WINDOWS_SUPERVISOR_POLL_INTERVAL);
        }
    };
    Ok(exit_code)
}

pub(super) fn spawn_low_integrity_windows_child(
    config: &ExecConfig<'_>,
    launch_program: &Path,
    containment: &ProcessContainment,
    cmd_args: &[String],
) -> Result<WindowsSupervisedChild> {
    let env_pairs = build_child_env(config);
    let mut environment_block = build_windows_environment_block(&env_pairs);
    let token = create_low_integrity_primary_token()?;
    let application_name: Vec<u16> = launch_program
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut command_line = build_command_line(launch_program, cmd_args);
    let current_dir: Vec<u16> = config
        .current_dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let startup_info = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..unsafe {
            // SAFETY: STARTUPINFOW is a plain FFI struct; zero initialization
            // is valid before filling the documented fields.
            std::mem::zeroed()
        }
    };
    let mut process_info = PROCESS_INFORMATION {
        ..unsafe {
            // SAFETY: PROCESS_INFORMATION is a plain FFI struct populated by
            // CreateProcessAsUserW.
            std::mem::zeroed()
        }
    };

    let created = unsafe {
        // SAFETY: All pointers either refer to valid, nul-terminated UTF-16
        // buffers or are null as documented by CreateProcessAsUserW.
        CreateProcessAsUserW(
            token.raw(),
            application_name.as_ptr(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            environment_block.as_mut_ptr() as *mut _,
            current_dir.as_ptr(),
            &startup_info,
            &mut process_info,
        )
    };
    if created == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to launch Windows low-integrity child process (GetLastError={})",
            unsafe { GetLastError() }
        )));
    }

    let process = OwnedHandle(process_info.hProcess);
    let thread = OwnedHandle(process_info.hThread);

    if let Err(err) = apply_process_handle_to_containment(containment, process.raw()) {
        terminate_suspended_process(process.raw(), "AssignProcessToJobObject failed");
        return Err(err);
    }
    resume_contained_process(process.raw(), thread.raw())?;
    Ok(WindowsSupervisedChild::Native {
        process,
        _thread: thread,
    })
}

pub(super) fn spawn_supervised_with_low_integrity(
    config: &ExecConfig<'_>,
    launch_program: &Path,
    containment: &ProcessContainment,
) -> Result<WindowsSupervisedChild> {
    let cmd_args = prepare_runtime_hardened_args(launch_program, &config.command[1..]);
    spawn_low_integrity_windows_child(config, launch_program, containment, &cmd_args)
}

pub(super) fn spawn_supervised_with_standard_token(
    config: &ExecConfig<'_>,
    launch_program: &Path,
    containment: &ProcessContainment,
) -> Result<WindowsSupervisedChild> {
    let cmd_args = prepare_runtime_hardened_args(launch_program, &config.command[1..]);
    spawn_windows_child_with_current_token(config, launch_program, containment, &cmd_args)
}

pub(super) fn spawn_windows_child_with_current_token(
    config: &ExecConfig<'_>,
    launch_program: &Path,
    containment: &ProcessContainment,
    cmd_args: &[String],
) -> Result<WindowsSupervisedChild> {
    let env_pairs = build_child_env(config);
    let mut environment_block = build_windows_environment_block(&env_pairs);
    let application_name: Vec<u16> = launch_program
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut command_line = build_command_line(launch_program, cmd_args);
    let current_dir: Vec<u16> = config
        .current_dir
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let startup_info = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        ..unsafe {
            // SAFETY: STARTUPINFOW is a plain FFI struct; zero initialization
            // is valid before filling the documented fields.
            std::mem::zeroed()
        }
    };
    let mut process_info = PROCESS_INFORMATION {
        ..unsafe {
            // SAFETY: PROCESS_INFORMATION is a plain FFI struct populated by CreateProcessW.
            std::mem::zeroed()
        }
    };

    let created = unsafe {
        // SAFETY: All pointers either refer to valid, nul-terminated UTF-16 buffers
        // or are null as documented by CreateProcessW.
        CreateProcessW(
            application_name.as_ptr(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            environment_block.as_mut_ptr() as *mut _,
            current_dir.as_ptr(),
            &startup_info,
            &mut process_info,
        )
    };
    if created == 0 {
        return Err(NonoError::SandboxInit(format!(
            "Failed to launch Windows child process with containment staging (GetLastError={})",
            unsafe { GetLastError() }
        )));
    }

    let process = OwnedHandle(process_info.hProcess);
    let thread = OwnedHandle(process_info.hThread);

    if let Err(err) = apply_process_handle_to_containment(containment, process.raw()) {
        terminate_suspended_process(process.raw(), "AssignProcessToJobObject failed");
        return Err(err);
    }
    resume_contained_process(process.raw(), thread.raw())?;
    Ok(WindowsSupervisedChild::Native {
        process,
        _thread: thread,
    })
}
