//! The shim launcher: reads its own embedded configuration and runs the target
//! with the caller's arguments, exactly once, invisible otherwise.
//!
//! It is built only as a console image. A shim for a GUI target is the same
//! binary with its PE subsystem field rewritten at creation time, so the
//! startup console policy is decided from the image, not the target.

#![windows_subsystem = "console"]

mod launch;
mod sys;

#[cfg(feature = "diagnostic")]
use hok_shim_spec::Config;
use hok_shim_spec::Utf16String;

const CTRL_C: u32 = 0;
const CTRL_BREAK: u32 = 1;
const CTRL_CLOSE: u32 = 2;
const CTRL_LOGOFF: u32 = 5;
const CTRL_SHUTDOWN: u32 = 6;

const EXIT_FAILURE: i32 = 1;

/// The flag that asks a diagnostic build to dump its configuration instead of
/// launching. Only compiled in when the feature is on, so a release shim can
/// never be talked into revealing or skipping its target.
#[cfg(feature = "diagnostic")]
const DIAGNOSTIC_FLAG: &str = "--hok-diagnostic";

fn main() {
    let gui = sys::own_subsystem_is_gui();
    let command_line = sys::command_line_units();
    let remainder = launch::caller_remainder(&command_line);

    #[cfg(feature = "diagnostic")]
    if launch::is_diagnostic(&command_line) {
        diagnostic();
    }

    let config = match sys::load_config() {
        Ok(config) => config,
        Err(message) => fail(&message),
    };

    let target = resolve_target(&config.target);
    let fixed: Vec<Utf16String> = config
        .args
        .iter()
        .map(|arg| sys::expand_environment_strings(arg.as_utf16()))
        .collect();
    let cwd = if config.cwd.is_empty() {
        None
    } else {
        Some(sys::expand_environment_strings(config.cwd.as_utf16()))
    };

    // A GUI-subsystem instance has no console of its own: with arguments it is
    // being used as a command-line tool, so it borrows the parent's; without
    // arguments it must not show a window at all.
    sys::gui_console_policy(gui, !remainder.is_empty() || !fixed.is_empty());
    sys::install_control_handler(Some(control_handler));

    for (name, value) in &config.env {
        if let Err(error) = sys::set_environment_variable(name.as_utf16(), value.as_utf16()) {
            sys::report_error(
                &format!("hok-shim: could not set {}", name.as_utf16()),
                error,
            );
        }
    }

    // The job ties the child's life to ours. Failing to make one is not fatal;
    // it just leaves the child ungoverned while we wait for it.
    let job = match sys::Job::create() {
        Ok(job) => Some(job),
        Err(error) => {
            sys::report_error("hok-shim: could not create job object", error);
            None
        }
    };

    let target_path = sys::WideString::new(target.as_utf16());
    let cwd = cwd.map(|cwd| sys::WideString::new(cwd.as_utf16()));
    let child = launch_child(
        &config,
        &target_path,
        cwd.as_ref(),
        &fixed,
        remainder,
        job.as_ref(),
    );

    let child = match child {
        Ok(child) => child,
        Err(error) => {
            sys::report_error("hok-shim: could not launch target", error);
            std::process::exit(EXIT_FAILURE);
        }
    };

    if let Err(error) = child.wait() {
        sys::report_error("hok-shim: could not wait for target", error);
        std::process::exit(EXIT_FAILURE);
    }

    match child.exit_code() {
        Ok(code) => std::process::exit(code as i32),
        Err(error) => {
            sys::report_error("hok-shim: could not read target exit code", error);
            std::process::exit(EXIT_FAILURE);
        }
    }
}

fn launch_child(
    config: &hok_shim_spec::Config,
    target: &sys::WideString,
    cwd: Option<&sys::WideString>,
    fixed: &[Utf16String],
    remainder: &[u16],
    job: Option<&sys::Job>,
) -> Result<sys::Child, u32> {
    if config.elevate {
        return sys::shell_execute_elevated(target, &elevated(fixed, remainder), cwd, job);
    }

    let command_line = launch::build_command_line(target.units(), fixed, remainder);
    let launch = sys::Launch {
        command_line: sys::WideString::new(command_line.as_utf16()),
        cwd,
    };

    match sys::create_process(launch, job) {
        // A target whose manifest demands elevation cannot be started
        // directly; the shell is the only way to raise the UAC prompt.
        Err(sys::ERROR_ELEVATION_REQUIRED) => {
            sys::shell_execute_elevated(target, &elevated(fixed, remainder), cwd, job)
        }
        other => other,
    }
}

fn elevated(fixed: &[Utf16String], remainder: &[u16]) -> sys::WideString {
    sys::WideString::new(launch::elevated_parameters(fixed, remainder).as_utf16())
}

/// Validate and expand the target path.
///
/// `%~dp0` is deliberately not expanded. libscoop resolves it when it creates
/// the shim, so by the time the configuration is embedded every path is
/// already absolute; a launcher-side expansion could only disagree with that.
fn resolve_target(target: &Utf16String) -> Utf16String {
    if launch::target_starts_with_separator(target.as_units()) {
        fail("target must not begin with a path separator");
    }
    sys::expand_environment_strings(target.as_utf16())
}

fn fail(message: &str) -> ! {
    sys::report(&format!("hok-shim: {message}"));
    std::process::exit(EXIT_FAILURE);
}

/// Swallow every console control event: the console belongs to the child, and
/// if the shim died on Ctrl+C the job object would take the child with it.
extern "system" fn control_handler(ctrl_type: u32) -> windows_sys::core::BOOL {
    match ctrl_type {
        CTRL_C | CTRL_BREAK | CTRL_CLOSE | CTRL_LOGOFF | CTRL_SHUTDOWN => 1,
        _ => 0,
    }
}

#[cfg(feature = "diagnostic")]
fn diagnostic() -> ! {
    match sys::load_config() {
        Ok(config) => {
            sys::report(&describe(&config));
            std::process::exit(0);
        }
        Err(message) => fail(&message),
    }
}

#[cfg(feature = "diagnostic")]
fn describe(config: &Config) -> String {
    let display = |text: &hok_shim_spec::Utf16Str| text.to_display_string();
    let mut out = format!(
        "schema: {}\ntarget: {}\ncwd: {}\nelevate: {}\n",
        hok_shim_spec::SCHEMA_VERSION,
        display(config.target.as_utf16()),
        display(config.cwd.as_utf16()),
        config.elevate,
    );
    for (index, arg) in config.args.iter().enumerate() {
        out.push_str(&format!("arg[{index}]: {}\n", display(arg.as_utf16())));
    }
    for (name, value) in &config.env {
        out.push_str(&format!(
            "env:{} = {}\n",
            display(name.as_utf16()),
            display(value.as_utf16())
        ));
    }
    out.trim_end().to_string()
}
