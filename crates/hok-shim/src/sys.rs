//! Win32 bindings and the small wrappers the launcher is built from.
//!
//! Every `unsafe` block in this crate lives here, so the launch policy in
//! `main.rs` reads as plain control flow.

#![allow(unsafe_code)]

use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};

use hok_shim_spec::{Utf16Str, Utf16String};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, GetLastError, HANDLE, WAIT_FAILED,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_CHAR, GetFileType, OPEN_EXISTING,
    ReadFile, WriteFile,
};
use windows_sys::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole, GetConsoleProcessList, GetStdHandle,
    PHANDLER_ROUTINE, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetConsoleCtrlHandler,
    WriteConsoleW,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    FORMAT_MESSAGE_FROM_SYSTEM, FORMAT_MESSAGE_IGNORE_INSERTS, FormatMessageW,
};
use windows_sys::Win32::System::Environment::{
    ExpandEnvironmentStringsW, GetCommandLineW, SetEnvironmentVariableW,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::LibraryLoader::{
    FindResourceW, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CreateProcessW, GetExitCodeProcess, GetStartupInfoW,
    INFINITE, PROCESS_INFORMATION, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOW,
    TerminateProcess, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;

pub const ERROR_ELEVATION_REQUIRED: u32 = 740;

/// `RT_RCDATA`, the predefined resource type for raw data.
const RT_RCDATA: u16 = 10;

const NO_HANDLE: HANDLE = null_mut();
const INVALID_HANDLE: HANDLE = -1isize as HANDLE;
const MAX_MODULE_PATH: usize = 32_768;

/// Resource name carrying the shim configuration.
const RESOURCE_NAME: &str = "HOK_SHIM_CONFIG";

/// A NUL-terminated UTF-16 buffer, ready to hand to a `PCWSTR` parameter.
pub struct WideString(Vec<u16>);

impl WideString {
    pub fn new(text: &Utf16Str) -> Self {
        let mut units = text.as_units().to_vec();
        units.push(0);
        Self(units)
    }

    pub fn from_str(text: &str) -> Self {
        Self::new(Utf16String::from_utf8(text).as_utf16())
    }

    pub fn from_units(units: &[u16]) -> Self {
        Self::new(Utf16Str::from_units(units))
    }

    pub fn as_ptr(&self) -> *const u16 {
        self.0.as_ptr()
    }

    /// The units without the trailing NUL.
    pub fn units(&self) -> &[u16] {
        &self.0[..self.0.len() - 1]
    }

    pub fn as_mut_ptr(&mut self) -> *mut u16 {
        self.0.as_mut_ptr()
    }
}

/// The calling process's arguments, without the program-name token.
pub fn command_line_units() -> Vec<u16> {
    let raw = unsafe { GetCommandLineW() };
    if raw.is_null() {
        return Vec::new();
    }

    let mut len = 0;
    while unsafe { *raw.add(len) } != 0 {
        len += 1;
    }
    let mut units = unsafe { std::slice::from_raw_parts(raw, len) }.to_vec();
    units.push(0);
    units
}

/// Read the calling module's image and answer whether it is GUI-subsystem.
pub fn own_subsystem_is_gui() -> bool {
    let mut path = WideString(Vec::new());
    if !module_file_name(&mut path) {
        return false;
    }

    let Some(image) = read_file_prefix(path.as_ptr(), 4096) else {
        return false;
    };
    crate::launch::is_gui_subsystem(&image)
}

fn module_file_name(out: &mut WideString) -> bool {
    let mut buffer = vec![0u16; MAX_MODULE_PATH];
    let written = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW(
            null_mut(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
        )
    };
    if written == 0 || written as usize >= buffer.len() {
        return false;
    }
    buffer.truncate(written as usize);
    *out = WideString::from_units(&buffer);
    true
}

fn read_file_prefix(path: *const u16, count: usize) -> Option<Vec<u8>> {
    let file = create_file(path, GENERIC_READ, FILE_SHARE_READ);
    if file == INVALID_HANDLE {
        return None;
    }

    let mut buffer = vec![0u8; count];
    let mut read = 0u32;
    let ok = unsafe {
        ReadFile(
            file,
            buffer.as_mut_ptr(),
            count as u32,
            &mut read,
            null_mut(),
        )
    };
    close_handle(file);
    if ok == 0 {
        return None;
    }
    buffer.truncate(read as usize);
    Some(buffer)
}

fn create_file(path: *const u16, access: u32, share: u32) -> HANDLE {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 0,
    };
    unsafe {
        CreateFileW(
            path,
            access,
            share,
            &attributes,
            OPEN_EXISTING,
            0,
            NO_HANDLE,
        )
    }
}

pub fn close_handle(handle: HANDLE) {
    if handle != NO_HANDLE && handle != INVALID_HANDLE {
        unsafe {
            CloseHandle(handle);
        }
    }
}

/// Load the configuration from the calling module's own resources.
///
/// The blob is one `RT_RCDATA` resource under the name `HOK_SHIM_CONFIG`.
/// Resource lookup by a NULL module handle searches the calling image, so this
/// never depends on the path the shim was started from.
pub fn load_config() -> Result<hok_shim_spec::Config, String> {
    let name = WideString::from_str(RESOURCE_NAME);
    let bytes = unsafe {
        let module = GetModuleHandleW(null());
        if module.is_null() {
            return Err(String::from("cannot address the calling module"));
        }

        let resource = FindResourceW(module, name.as_ptr(), RT_RCDATA as usize as *const u16);
        if resource.is_null() {
            return Err(String::from("no configuration resource"));
        }

        let size = SizeofResource(module, resource) as usize;
        let loaded = LoadResource(module, resource);
        if loaded.is_null() || size == 0 {
            return Err(String::from("could not load the configuration resource"));
        }

        let data = LockResource(loaded);
        if data.is_null() {
            return Err(String::from("could not lock the configuration resource"));
        }

        std::slice::from_raw_parts(data as *const u8, size).to_vec()
    };

    hok_shim_spec::decode(&bytes).map_err(|error| format!("invalid shim configuration: {error:?}"))
}
/// Expand `%VAR%` in place, leaving unknown variables untouched.
pub fn expand_environment_strings(text: &Utf16Str) -> Utf16String {
    let source = WideString::new(text);
    let mut buffer = vec![0u16; 512];

    let mut needed =
        unsafe { ExpandEnvironmentStringsW(source.as_ptr(), buffer.as_mut_ptr(), 512) };
    if needed == 0 {
        return Utf16String::from(text);
    }
    if needed as usize > buffer.len() {
        buffer.resize(needed as usize, 0);
        needed = unsafe {
            ExpandEnvironmentStringsW(source.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32)
        };
        if needed == 0 || needed as usize > buffer.len() {
            return Utf16String::from(text);
        }
    }

    buffer.truncate(needed as usize - 1);
    Utf16String::from_units(buffer)
}

pub fn set_environment_variable(name: &Utf16Str, value: &Utf16Str) -> Result<(), u32> {
    let name = WideString::new(name);
    let value = WideString::new(value);
    let ok = unsafe { SetEnvironmentVariableW(name.as_ptr(), value.as_ptr()) };
    if ok == 0 { Err(last_error()) } else { Ok(()) }
}

pub fn last_error() -> u32 {
    unsafe { GetLastError() }
}

pub fn gui_console_policy(gui: bool, passes_arguments: bool) {
    if !gui {
        return;
    }
    if passes_arguments {
        unsafe {
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    } else {
        unsafe {
            FreeConsole();
        }
    }
}

pub fn has_console() -> bool {
    let mut process = 0u32;
    unsafe { GetConsoleProcessList(&mut process, 1) > 0 }
}

pub fn install_control_handler(handler: PHANDLER_ROUTINE) {
    unsafe {
        SetConsoleCtrlHandler(handler, 1);
    }
}

/// A job object that kills its members when the last handle to it closes.
pub struct Job(HANDLE);

impl Job {
    pub fn create() -> Result<Self, u32> {
        let handle = unsafe { CreateJobObjectW(null(), null()) };
        if handle == NO_HANDLE {
            return Err(last_error());
        }

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags =
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
        let ok = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            let error = last_error();
            close_handle(handle);
            return Err(error);
        }
        Ok(Self(handle))
    }

    pub fn assign(&self, process: HANDLE) -> Result<(), u32> {
        let ok = unsafe { AssignProcessToJobObject(self.0, process) };
        if ok == 0 { Err(last_error()) } else { Ok(()) }
    }

    pub fn terminate(&self, code: u32) {
        unsafe {
            TerminateJobObject(self.0, code);
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        close_handle(self.0);
    }
}

/// A handle to the launched child's process.
pub struct Child(HANDLE);

impl Child {
    pub fn is_null(&self) -> bool {
        self.0 == NO_HANDLE
    }

    pub fn raw(&self) -> HANDLE {
        self.0
    }

    pub fn wait(&self) -> Result<(), u32> {
        let result = unsafe { WaitForSingleObject(self.0, INFINITE) };
        if result == WAIT_FAILED {
            Err(last_error())
        } else {
            Ok(())
        }
    }

    pub fn exit_code(&self) -> Result<u32, u32> {
        let mut code = 0u32;
        let ok = unsafe { GetExitCodeProcess(self.0, &mut code) };
        if ok == 0 { Err(last_error()) } else { Ok(code) }
    }

    pub fn terminate(&self, code: u32) {
        unsafe {
            TerminateProcess(self.0, code);
        }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        close_handle(self.0);
    }
}

pub struct Launch<'a> {
    pub command_line: WideString,
    pub cwd: Option<&'a WideString>,
}

/// Start the child suspended, inside the job when one exists, then resume it.
pub fn create_process(launch: Launch<'_>, job: Option<&Job>) -> Result<Child, u32> {
    let startup = startup_info();
    let inheritable = i32::from(startup.dwFlags & STARTF_USESTDHANDLES != 0);

    let mut flags = CREATE_SUSPENDED;
    if !has_console() {
        flags |= CREATE_NO_WINDOW;
    }

    let mut command_line = launch.command_line;
    let target = WideString::from_units(
        // The program name is the target the command line starts with; letting
        // `CreateProcessW` search PATH for a bare name would launch something
        // other than the resolved target.
        &command_line.units()[..program_units(command_line.units())],
    );
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let created = unsafe {
        CreateProcessW(
            target.as_ptr(),
            command_line.as_mut_ptr(),
            null(),
            null(),
            inheritable,
            flags,
            null(),
            launch.cwd.map_or(null(), WideString::as_ptr),
            &startup,
            &mut info,
        )
    };
    if created == 0 {
        return Err(last_error());
    }

    let child = Child(info.hProcess);
    let thread = info.hThread;

    if let Some(job) = job
        && let Err(error) = job.assign(child.raw())
    {
        child.terminate(1);
        close_handle(thread);
        return Err(error);
    }

    let resumed = unsafe { ResumeThread(thread) };
    close_handle(thread);
    if resumed == u32::MAX {
        // A child that never resumes would hold its image until we exit, so
        // kill it directly rather than wait on a process that cannot start.
        let error = last_error();
        match job {
            Some(job) => job.terminate(1),
            None => child.terminate(1),
        }
        return Err(error);
    }

    Ok(child)
}

/// Launch through the shell's `runas` verb, which is the only way to reach a
/// UAC prompt. `parameters` excludes the program path, which `lpFile` carries.
pub fn shell_execute_elevated(
    target: &WideString,
    parameters: &WideString,
    cwd: Option<&WideString>,
    job: Option<&Job>,
) -> Result<Child, u32> {
    let verb = WideString::from_str("runas");
    let empty_parameters = WideString::from_units(&[]);

    let parameters = if parameters.0.len() <= 1 {
        &empty_parameters
    } else {
        parameters
    };

    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS;
    info.lpVerb = verb.as_ptr();
    info.lpFile = target.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.lpDirectory = cwd.map_or(null(), WideString::as_ptr);
    info.nShow = SW_SHOW;

    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        // The shell reports its own failures as small SE_ERR_* values where
        // GetLastError would otherwise be meaningless.
        let code = info.hInstApp as usize;
        let error = if (1..=32).contains(&code) {
            code as u32
        } else {
            last_error()
        };
        return Err(error);
    }

    let child = Child(info.hProcess);
    if child.is_null() {
        return Err(0);
    }
    if let Some(job) = job
        && let Err(error) = job.assign(child.raw())
    {
        child.terminate(1);
        return Err(error);
    }
    Ok(child)
}

fn startup_info() -> STARTUPINFOW {
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = size_of::<STARTUPINFOW>() as u32;
    unsafe {
        GetStartupInfoW(&mut startup);
    }
    seed_standard_handles(&mut startup);
    startup
}

/// Fill empty standard-handle slots so the child inherits working streams.
///
/// A creator that sets `STARTF_USESTDHANDLES` but leaves a slot null (Go's
/// `os/exec` does) would otherwise hand the child a closed stream.
fn seed_standard_handles(startup: &mut STARTUPINFOW) {
    for (slot, which) in [
        (&mut startup.hStdInput, STD_INPUT_HANDLE),
        (&mut startup.hStdOutput, STD_OUTPUT_HANDLE),
        (&mut startup.hStdError, STD_ERROR_HANDLE),
    ] {
        if *slot == NO_HANDLE || *slot == INVALID_HANDLE {
            let inherited = unsafe { GetStdHandle(which) };
            if inherited != NO_HANDLE && inherited != INVALID_HANDLE {
                *slot = inherited;
            }
        }
    }

    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let console_in = WideString::from_str("CONIN$");
    let console_out = WideString::from_str("CONOUT$");
    let mut replaced = false;

    for (slot, name, access, share) in [
        (
            &mut startup.hStdInput,
            &console_in,
            GENERIC_READ,
            FILE_SHARE_READ,
        ),
        (
            &mut startup.hStdOutput,
            &console_out,
            GENERIC_WRITE,
            FILE_SHARE_WRITE,
        ),
        (
            &mut startup.hStdError,
            &console_out,
            GENERIC_WRITE,
            FILE_SHARE_WRITE,
        ),
    ] {
        if *slot != NO_HANDLE && *slot != INVALID_HANDLE {
            continue;
        }
        let opened = unsafe {
            CreateFileW(
                name.as_ptr(),
                access,
                share,
                &attributes,
                OPEN_EXISTING,
                0,
                NO_HANDLE,
            )
        };
        if opened != INVALID_HANDLE {
            *slot = opened;
            replaced = true;
        }
    }

    if replaced {
        startup.dwFlags |= STARTF_USESTDHANDLES;
    }
    // The opened handles live as long as the process; closing them would be a bug.
}

/// Report a diagnostic on stderr, bypassing stdio so nothing is buffered behind
/// the child's output.
pub fn report(message: &str) {
    let handle = unsafe { GetStdHandle(STD_ERROR_HANDLE) };
    if handle == NO_HANDLE || handle == INVALID_HANDLE {
        return;
    }

    let mut units: Vec<u16> = Utf16String::from_utf8(message).as_units().to_vec();
    units.push(unit(b'\n'));

    if unsafe { GetFileType(handle) } == FILE_TYPE_CHAR {
        let mut written = 0u32;
        unsafe {
            WriteConsoleW(
                handle,
                units.as_ptr(),
                units.len() as u32,
                &mut written,
                null(),
            );
        }
    } else {
        let bytes = String::from_utf16_lossy(&units);
        let mut written = 0u32;
        unsafe {
            WriteFile(
                handle,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut written,
                null_mut(),
            );
        }
    }
}

/// Report a diagnostic followed by the system message for a Win32 error.
pub fn report_error(message: &str, error: u32) {
    report(&format!("{message}: {}", system_message(error)));
}

fn system_message(error: u32) -> String {
    let mut buffer = [0u16; 512];
    let written = unsafe {
        FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            null(),
            error,
            0,
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            null(),
        )
    };
    if written == 0 {
        return format!("error {error}");
    }

    let text = String::from_utf16_lossy(&buffer[..written as usize]);
    format!("{} (error {error})", text.trim_end())
}

/// Length of the leading quoted target in a command line, using the same rules
/// as argument parsing: blanks separate tokens only outside quotes.
fn program_units(command_line: &[u16]) -> usize {
    let mut index = 0;
    let mut in_quotes = false;
    while index < command_line.len() {
        let current = command_line[index];
        if current == unit(b'"') {
            in_quotes = !in_quotes;
        } else if !in_quotes && (current == unit(b' ') || current == unit(b'\t')) {
            break;
        }
        index += 1;
    }
    index
}

fn unit(byte: u8) -> u16 {
    u16::from(byte)
}
