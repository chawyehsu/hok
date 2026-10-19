//! Pure launch decisions: command-line assembly, quoting, and image parsing.
//!
//! Nothing here touches Win32, so all of it is unit-testable off-process.
//!
//! Fixed arguments arrive as individual values, not as one `args` string:
//! [`hok_shim_spec::to_text`] joins them with a space and no quoting escape, so
//! a value containing spaces could not survive being parsed back out of the
//! `.shim` text. libscoop is expected to pass the values it already resolved.

use hok_shim_spec::Utf16String;

const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D;
const IMAGE_NT_SIGNATURE: u32 = 0x0000_4550;
const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;

/// Offset of `e_lfanew`, which points at the PE signature.
const DOS_LFANEW: usize = 0x3C;

/// Offset of `Subsystem` from the PE signature: 4-byte signature plus the
/// 20-byte COFF header plus 0x44 bytes of the optional header.
const PE_SUBSYSTEM: usize = 0x5C;

/// Upper bound on a plausible `e_lfanew`. The headers always live in the first
/// page, so anything beyond it is a bogus offset rather than a valid image.
const PE_PAGE: usize = 0x1000;

/// Whether an image's subsystem is GUI. Anything unparseable counts as console,
/// because a console image misread as GUI would lose its console before it
/// could hand it to the child.
pub fn is_gui_subsystem(image: &[u8]) -> bool {
    let Some(subsystem) = read_subsystem(image) else {
        return false;
    };
    subsystem == IMAGE_SUBSYSTEM_WINDOWS_GUI
}

fn read_subsystem(image: &[u8]) -> Option<u16> {
    let dos_signature = image.get(..2)?;
    if u16::from_le_bytes([dos_signature[0], dos_signature[1]]) != IMAGE_DOS_SIGNATURE {
        return None;
    }

    let lfanew = image.get(DOS_LFANEW..DOS_LFANEW + 4)?;
    let pe = i32::from_le_bytes([lfanew[0], lfanew[1], lfanew[2], lfanew[3]]);
    let pe = usize::try_from(pe).ok()?;
    if pe == 0 || pe >= PE_PAGE {
        return None;
    }

    let signature = image.get(pe..pe + 4)?;
    if u32::from_le_bytes([signature[0], signature[1], signature[2], signature[3]])
        != IMAGE_NT_SIGNATURE
    {
        return None;
    }

    let subsystem = image.get(pe + PE_SUBSYSTEM..pe + PE_SUBSYSTEM + 2)?;
    Some(u16::from_le_bytes([subsystem[0], subsystem[1]]))
}

/// The caller's arguments as they appeared on the command line, minus the
/// program-name token.
///
/// The text is taken verbatim rather than parsed and re-quoted: re-quoting
/// could only lose fidelity, and the child interprets the line with the same
/// rules we would have to reimplement.
pub fn caller_remainder(command_line: &[u16]) -> &[u16] {
    let end = code_units_len(command_line);
    let line = &command_line[..end];
    let after_program = program_token_end(line);
    let mut start = after_program;
    while start < line.len() && is_blank(line[start]) {
        start += 1;
    }
    &line[start..]
}

/// The index one past the program-name token, or the line length when no
/// program name is present.
///
/// This mirrors `CommandLineToArgvW`, not a plain shell split: outside quotes
/// whitespace separates arguments, and an opening quote starts a run in which
/// only another quote is special.
fn program_token_end(line: &[u16]) -> usize {
    let mut index = 0;
    while index < line.len() && is_blank(line[index]) {
        index += 1;
    }

    let mut in_quotes = false;
    while index < line.len() {
        let unit = line[index];
        if unit == unit_of(b'"') {
            in_quotes = !in_quotes;
        } else if is_blank(unit) && !in_quotes {
            break;
        }
        index += 1;
    }
    index
}

/// Whether the command line asks a diagnostic build to dump its configuration.
///
/// The flag has to be the first argument after the program name, so it cannot
/// be reached by a caller whose target merely happens to take the same
/// argument.
#[cfg(feature = "diagnostic")]
pub fn is_diagnostic(command_line: &[u16]) -> bool {
    let flag: Vec<u16> = crate::DIAGNOSTIC_FLAG.encode_utf16().collect();
    caller_remainder(command_line).starts_with(&flag)
}
/// Whether the target begins with a path separator.
///
/// A leading separator anchors the target to the current drive, so one shim
/// would resolve to different targets depending on where it was invoked.
pub fn target_starts_with_separator(target: &[u16]) -> bool {
    matches!(target.first(), Some(&unit) if is_separator(unit))
}

/// Quote one argument so a `CommandLineToArgvW` parser reads it back intact.
pub fn quote_arg(arg: &[u16]) -> Utf16String {
    if arg.is_empty() {
        return Utf16String::from_utf8("\"\"");
    }

    if !arg
        .iter()
        .any(|&unit| is_blank(unit) || unit == unit_of(b'"'))
    {
        return Utf16String::from_units(arg.to_vec());
    }

    let mut out: Vec<u16> = Vec::with_capacity(arg.len() + 8);
    out.push(unit_of(b'"'));

    let mut index = 0;
    while index < arg.len() {
        if arg[index] != unit_of(b'\\') {
            if arg[index] == unit_of(b'"') {
                out.push(unit_of(b'\\'));
            }
            out.push(arg[index]);
            index += 1;
            continue;
        }

        let run_start = index;
        while index < arg.len() && arg[index] == unit_of(b'\\') {
            index += 1;
        }
        let run = index - run_start;
        let followed_by_quote = arg.get(index) == Some(&unit_of(b'"'));

        // Backslashes are literal unless one lands right before a quote or the
        // closing quote we are about to add, where the parser needs them
        // doubled so it un-escapes back to the original count.
        let count = if followed_by_quote || index == arg.len() {
            run * 2
        } else {
            run
        };
        out.extend(std::iter::repeat_n(unit_of(b'\\'), count));
        if followed_by_quote {
            out.push(unit_of(b'\\'));
            out.push(unit_of(b'"'));
            index += 1;
        }
    }

    out.push(unit_of(b'"'));
    Utf16String::from_units(out)
}

/// Assemble the child command line from the quoted target, the fixed
/// arguments, and the caller's own argument text.
pub fn build_command_line(target: &[u16], fixed: &[Utf16String], remainder: &[u16]) -> Utf16String {
    let mut out = quote_arg(target).as_units().to_vec();
    for arg in fixed {
        out.push(unit_of(b' '));
        out.extend_from_slice(quote_arg(arg.as_units()).as_units());
    }
    if !remainder.is_empty() {
        out.push(unit_of(b' '));
        out.extend_from_slice(remainder);
    }
    Utf16String::from_units(out)
}

/// The parameter text for an elevated launch: fixed arguments plus the
/// caller's remainder, with the executable path left to `lpFile`.
pub fn elevated_parameters(fixed: &[Utf16String], remainder: &[u16]) -> Utf16String {
    let mut out: Vec<u16> = Vec::new();
    for arg in fixed {
        if !out.is_empty() {
            out.push(unit_of(b' '));
        }
        out.extend_from_slice(quote_arg(arg.as_units()).as_units());
    }
    if !remainder.is_empty() {
        if !out.is_empty() {
            out.push(unit_of(b' '));
        }
        out.extend_from_slice(remainder);
    }
    Utf16String::from_units(out)
}

fn unit_of(byte: u8) -> u16 {
    u16::from(byte)
}

fn is_blank(unit: u16) -> bool {
    unit == unit_of(b' ') || unit == unit_of(b'\t')
}

fn is_separator(unit: u16) -> bool {
    unit == unit_of(b'\\') || unit == unit_of(b'/')
}

/// Length up to the first NUL, for a C-style command line.
fn code_units_len(units: &[u16]) -> usize {
    units
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(units.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn quote(text: &str) -> String {
        quote_arg(&wide(text)).as_utf16().to_display_string()
    }

    #[test]
    fn empty_argument_quotes_to_empty_string() {
        assert_eq!(quote(""), "\"\"");
    }

    #[test]
    fn plain_argument_is_not_quoted() {
        assert_eq!(quote("--flag"), "--flag");
        assert_eq!(quote("C:\\a\\b.exe"), "C:\\a\\b.exe");
    }

    #[test]
    fn spaces_and_tabs_force_quoting() {
        assert_eq!(quote("a b"), "\"a b\"");
        assert_eq!(quote("a\tb"), "\"a\tb\"");
    }

    #[test]
    fn backslashes_before_quote_are_doubled() {
        // 1 backslash + quote -> 2 backslashes + escaped quote.
        assert_eq!(quote(r#"a\"b"#), r#""a\\\"b""#);
        // 2 backslashes + quote, the 2n case.
        assert_eq!(quote(r#"a\\"b"#), r#""a\\\\\"b""#);
        // 3 backslashes + quote, the 2n+1 case.
        assert_eq!(quote(r#"a\\\"b"#), r#""a\\\\\\\"b""#);
    }

    #[test]
    fn trailing_backslashes_are_doubled() {
        assert_eq!(quote("a b\\"), "\"a b\\\\\"");
        assert_eq!(quote("a\t\\\\"), "\"a\t\\\\\\\\\"");
    }

    #[test]
    fn lone_quote_is_escaped() {
        assert_eq!(quote("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn unquoted_trailing_backslashes_are_left_alone() {
        assert_eq!(quote("C:\\a\\"), "C:\\a\\");
    }

    #[test]
    fn remainder_skips_a_bare_program_name() {
        let line = wide("shim.exe --flag a");
        assert_eq!(
            String::from_utf16_lossy(caller_remainder(&line)),
            "--flag a"
        );
    }

    #[test]
    fn remainder_skips_a_quoted_program_name() {
        let line = wide(r#""C:\Program Files\shim.exe" one two"#);
        assert_eq!(String::from_utf16_lossy(caller_remainder(&line)), "one two");
    }

    #[test]
    fn remainder_keeps_quotes_and_embedded_spaces() {
        let line = wide(r#"shim.exe "a b" c"#);
        assert_eq!(
            String::from_utf16_lossy(caller_remainder(&line)),
            r#""a b" c"#
        );
    }

    #[test]
    fn remainder_stops_at_nul() {
        let line = [wide("shim.exe x").as_slice(), &[0u16]].concat();
        assert_eq!(String::from_utf16_lossy(caller_remainder(&line)), "x");
    }

    #[test]
    fn remainder_is_empty_without_arguments() {
        assert!(caller_remainder(&wide("shim.exe")).is_empty());
        assert!(caller_remainder(&wide("")).is_empty());
        assert!(caller_remainder(&wide("shim.exe   ")).is_empty());
    }

    #[test]
    fn target_separators_are_rejected() {
        assert!(target_starts_with_separator(&wide(r"\tool.exe")));
        assert!(target_starts_with_separator(&wide("/tool.exe")));
        assert!(!target_starts_with_separator(&wide(r"C:\tool.exe")));
        assert!(!target_starts_with_separator(&wide(r"tool.exe")));
        assert!(!target_starts_with_separator(&[]));
    }

    #[test]
    fn command_line_joins_quoted_parts() {
        // Fixed arguments are already individual values: the writer split the
        // `args` line, so a value keeps no wrapping quotes of its own.
        let fixed = vec![
            Utf16String::from("--ini"),
            Utf16String::from(r"C:\a b\cli.ini"),
        ];
        let built = build_command_line(
            &wide(r"C:\Program Files\tool.exe"),
            &fixed,
            &wide(r#""user arg""#),
        );
        assert_eq!(
            built.as_utf16().to_display_string(),
            r#""C:\Program Files\tool.exe" --ini "C:\a b\cli.ini" "user arg""#
        );
    }

    #[test]
    fn command_line_omits_empty_remainder() {
        let built = build_command_line(&wide("tool.exe"), &[], &[]);
        assert_eq!(built.as_utf16().to_display_string(), "tool.exe");
    }

    #[test]
    fn elevated_parameters_leave_out_the_program() {
        let fixed = vec![Utf16String::from("-E")];
        let params = elevated_parameters(&fixed, &wide("x"));
        assert_eq!(params.as_utf16().to_display_string(), "-E x");
        assert!(elevated_parameters(&[], &[]).is_empty());
    }

    fn synthetic_exe(pe_offset: usize, subsystem: u16) -> Vec<u8> {
        let mut image = vec![0u8; pe_offset + 0x60];
        image[0..2].copy_from_slice(&IMAGE_DOS_SIGNATURE.to_le_bytes());
        image[0x3C..0x40].copy_from_slice(&(pe_offset as i32).to_le_bytes());
        image[pe_offset..pe_offset + 4].copy_from_slice(&IMAGE_NT_SIGNATURE.to_le_bytes());
        image[pe_offset + 0x5C..pe_offset + 0x5E].copy_from_slice(&subsystem.to_le_bytes());
        image
    }

    #[test]
    fn subsystem_is_read_from_the_optional_header() {
        assert!(!is_gui_subsystem(&synthetic_exe(0x80, 3)));
        assert!(is_gui_subsystem(&synthetic_exe(0x100, 2)));
    }

    #[test]
    fn malformed_images_are_not_gui() {
        assert!(!is_gui_subsystem(&[]));
        assert!(!is_gui_subsystem(b"MZ"));

        let mut wrong_dos = synthetic_exe(0x80, 2);
        wrong_dos[0] = b'Z';
        assert!(!is_gui_subsystem(&wrong_dos));

        let mut wrong_nt = synthetic_exe(0x80, 2);
        wrong_nt[0x80] = 0;
        assert!(!is_gui_subsystem(&wrong_nt));

        let mut truncated = synthetic_exe(0x80, 2);
        truncated.truncate(0x80 + 0x5C);
        assert!(!is_gui_subsystem(&truncated));

        let mut bogus_offset = synthetic_exe(0x80, 2);
        bogus_offset[0x3C..0x40].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(!is_gui_subsystem(&bogus_offset));

        let mut out_of_page = synthetic_exe(0x80, 2);
        out_of_page[0x3C..0x40].copy_from_slice(&0x2000i32.to_le_bytes());
        assert!(!is_gui_subsystem(&out_of_page));
    }
}
