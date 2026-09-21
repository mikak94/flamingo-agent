use std::ffi::OsStr;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::ptr;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, HLOCAL, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1, SE_FILE_OBJECT,
    SetNamedSecurityInfoW,
};
use windows_sys::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetTokenInformation,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, TOKEN_ELEVATION, TOKEN_QUERY,
    TokenElevation,
};
use windows_sys::Win32::System::Console::{
    CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler,
};
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::error::{Error, Result};
use crate::service::ServiceCommand;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// `D:P` discards inherited entries; then full access to Administrators (BA)
/// and SYSTEM (SY) and nothing else.
const LOG_FILE_SDDL: &str = "D:P(A;;FA;;;BA)(A;;FA;;;SY)";

/// `WorkingSetSize` is the Windows equivalent of RSS.
pub fn process_rss_bytes() -> Result<u64> {
    let mut counters: PROCESS_MEMORY_COUNTERS = unsafe { mem::zeroed() };
    counters.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: `counters` is correctly sized; the pseudo-handle needs no closing.
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) };
    if ok == 0 {
        return Err(Error::last_os("GetProcessMemoryInfo"));
    }
    Ok(counters.WorkingSetSize as u64)
}

pub fn is_elevated() -> Result<bool> {
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: `token` is a valid out-parameter; closed below.
    let ok = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
    if ok == 0 {
        return Err(Error::last_os("OpenProcessToken"));
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut returned: u32 = 0;
    // SAFETY: the buffer matches the TokenElevation information class.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut core::ffi::c_void,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    };
    let err = (ok == 0).then(|| Error::last_os("GetTokenInformation(TokenElevation)"));
    // SAFETY: `token` came from OpenProcessToken and is not used afterwards.
    unsafe { CloseHandle(token) };

    match err {
        Some(e) => Err(e),
        None => Ok(elevation.TokenIsElevated != 0),
    }
}

/// Replaces the file's DACL with LOG_FILE_SDDL.
pub fn restrict_to_admins(path: &Path) -> Result<()> {
    let sddl = to_wide(OsStr::new(LOG_FILE_SDDL));
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    // SAFETY: `sddl` is NUL-terminated; `descriptor` is LocalFree'd below.
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(Error::last_os(
            "ConvertStringSecurityDescriptorToSecurityDescriptorW",
        ));
    }
    let result = apply_dacl(path, descriptor);
    // SAFETY: allocated by the conversion call above.
    unsafe { LocalFree(descriptor as HLOCAL) };
    result
}

fn apply_dacl(path: &Path, descriptor: PSECURITY_DESCRIPTOR) -> Result<()> {
    // maybe switch to native rust Permissions for windows, once stable
    // https://github.com/rust-lang/rust/issues/152956

    let mut dacl_present: i32 = 0;
    let mut dacl: *mut ACL = ptr::null_mut();
    let mut dacl_defaulted: i32 = 0;
    // SAFETY: `descriptor` is a valid self-relative security descriptor.
    let ok = unsafe {
        GetSecurityDescriptorDacl(
            descriptor,
            &mut dacl_present,
            &mut dacl,
            &mut dacl_defaulted,
        )
    };
    if ok == 0 {
        return Err(Error::last_os("GetSecurityDescriptorDacl"));
    }
    if dacl_present == 0 || dacl.is_null() {
        return Err(Error::other("no DACL in the security descriptor"));
    }

    let wide_path = to_wide(path.as_os_str());
    // SAFETY: `wide_path` is NUL-terminated; `dacl` points into `descriptor`,
    // which outlives this call.
    let status = unsafe {
        SetNamedSecurityInfoW(
            wide_path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            ptr::null_mut(),
            ptr::null_mut(),
            dacl,
            ptr::null(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(Error::os("SetNamedSecurityInfoW", status));
    }
    Ok(())
}

/// The child inherits our token; as the LocalSystem service that makes it
/// administrative. Unelevated in the foreground it gets an unelevated child.
pub fn spawn_child(exe: &Path, args: &[&OsStr], console: bool) -> Result<Child> {
    let mut command = Command::new(exe);
    command.args(args);
    if console {
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        // stderr is piped so the child's complaints reach the agent log.
        command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
    }
    Ok(command.spawn()?)
}

/// Foreground only; under the SCM the stop comes via the control handler.
pub fn install_shutdown_handler(commands: Sender<ServiceCommand>) -> Result<()> {
    let _ = COMMANDS.set(Mutex::new(commands));
    // SAFETY: `console_handler` has the required signature and static lifetime.
    let ok = unsafe { SetConsoleCtrlHandler(Some(console_handler), 1) };
    if ok == 0 {
        return Err(Error::last_os("SetConsoleCtrlHandler"));
    }
    Ok(())
}

/// `Sender` is not `Sync`, hence the mutex.
static COMMANDS: OnceLock<Mutex<Sender<ServiceCommand>>> = OnceLock::new();

unsafe extern "system" fn console_handler(ctrl_type: u32) -> i32 {
    match ctrl_type {
        CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT | CTRL_SHUTDOWN_EVENT => {
            if let Some(Ok(commands)) = COMMANDS.get().map(Mutex::lock) {
                let _ = commands.send(ServiceCommand::Stop);
            }
            1
        }
        _ => 0,
    }
}

fn to_wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(std::iter::once(0)).collect()
}
