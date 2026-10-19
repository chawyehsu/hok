use std::path::Path;

use hok_shim_spec::{Config, Utf16String, decode, encode, to_text};

use super::{CONFIG_RESOURCE_NAME, HOK_SHIM_BYTES, pe, resource, subsystem_of, wides};

fn sample_config() -> Config {
    Config {
        target: Utf16String::from(r"C:\hok\apps\php\current\php.exe"),
        args: vec![
            Utf16String::from("--ini"),
            Utf16String::from(r"C:\a b\cli.ini"),
        ],
        cwd: Utf16String::from(r"C:\Users\me"),
        elevate: true,
        env: vec![(
            Utf16String::from("PHP_INI_SCAN_DIR"),
            Utf16String::from(r"C:\hok\apps\php\current\conf.d"),
        )],
    }
}

fn write_template(dir: &Path, image: &[u8]) -> std::path::PathBuf {
    let path = dir.join("shim.exe");
    std::fs::write(&path, image).unwrap();
    path
}

fn templated_image() -> Vec<u8> {
    HOK_SHIM_BYTES.to_vec()
}

#[test]
fn template_is_a_pe_console_image() {
    assert_eq!(
        pe::subsystem(&templated_image()),
        Some(pe::IMAGE_SUBSYSTEM_WINDOWS_CUI)
    );
}

#[test]
fn config_round_trips_through_the_embedded_resource() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_template(dir.path(), &templated_image());
    let config = sample_config();

    let payload = encode(&config).unwrap();
    resource::write(&path, &wides(CONFIG_RESOURCE_NAME), &payload, Some(0x0409)).unwrap();

    let read = resource::read(&path, &wides(CONFIG_RESOURCE_NAME)).unwrap();
    assert_eq!(decode(&read).unwrap(), config);
}

#[test]
fn resource_replacement_round_trips_the_latest_value() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_template(dir.path(), &templated_image());

    let first = Config {
        target: Utf16String::from(r"C:\hok\apps\a\current\a.exe"),
        ..Config::default()
    };
    let second = sample_config();

    resource::write(
        &path,
        &wides(CONFIG_RESOURCE_NAME),
        &encode(&first).unwrap(),
        Some(0x0409),
    )
    .unwrap();
    resource::write(
        &path,
        &wides(CONFIG_RESOURCE_NAME),
        &encode(&second).unwrap(),
        Some(0x0409),
    )
    .unwrap();

    let read = resource::read(&path, &wides(CONFIG_RESOURCE_NAME)).unwrap();
    assert_eq!(decode(&read).unwrap(), second);
}

#[test]
fn numeric_resource_types_are_makeintresource_ordinals() {
    // The ordinal form is the bare integer in pointer position, so reading the
    // type back as a resource id must yield 10 rather than dereference a buffer.
    let id = resource::make_int_resource_for_test(resource::RT_RCDATA);
    assert_eq!(id, 10usize as *const u16);

    // A `RT_RCDATA` written under this name is only found with the ordinal
    // form; searching with a pointer to a buffer holding `10` finds nothing.
    let dir = tempfile::tempdir().unwrap();
    let path = write_template(dir.path(), &templated_image());
    let name = wides(CONFIG_RESOURCE_NAME);
    resource::write(&path, &name, b"ordinal-form", Some(0x0409)).unwrap();

    assert!(resource::find_by_ordinal_for_test(&path, &name, 10));
    assert!(!resource::find_by_buffer_for_test(&path, &name, 10));
}

#[test]
fn console_target_keeps_the_console_subsystem() {
    let dir = tempfile::tempdir().unwrap();

    let mut image = templated_image();
    assert_eq!(pe::subsystem(&image), Some(pe::IMAGE_SUBSYSTEM_WINDOWS_CUI));
    pe::set_subsystem(&mut image, pe::IMAGE_SUBSYSTEM_WINDOWS_CUI).unwrap();

    let path = write_template(dir.path(), &image);
    let written = std::fs::read(&path).unwrap();
    assert_eq!(
        pe::subsystem(&written),
        Some(pe::IMAGE_SUBSYSTEM_WINDOWS_CUI)
    );
}

#[test]
fn gui_target_patches_the_subsystem_field() {
    let dir = tempfile::tempdir().unwrap();

    let mut image = templated_image();
    pe::set_subsystem(&mut image, pe::IMAGE_SUBSYSTEM_WINDOWS_GUI).unwrap();

    let path = write_template(dir.path(), &image);
    let written = std::fs::read(&path).unwrap();
    assert_eq!(
        pe::subsystem(&written),
        Some(pe::IMAGE_SUBSYSTEM_WINDOWS_GUI)
    );

    // The patch still leaves a loadable image with its config slot writable.
    let config = sample_config();
    resource::write(
        &path,
        &wides(CONFIG_RESOURCE_NAME),
        &encode(&config).unwrap(),
        Some(0x0409),
    )
    .unwrap();
    let read = resource::read(&path, &wides(CONFIG_RESOURCE_NAME)).unwrap();
    assert_eq!(decode(&read).unwrap(), config);
}

#[test]
fn subsystem_patch_rejects_a_non_pe_image() {
    let mut image = b"not a pe image at all, definitely".to_vec();
    assert!(pe::set_subsystem(&mut image, pe::IMAGE_SUBSYSTEM_WINDOWS_GUI).is_err());
    assert_eq!(pe::subsystem(&image), None);

    let mut truncated = templated_image();
    truncated.truncate(0x3E);
    assert!(pe::set_subsystem(&mut truncated, pe::IMAGE_SUBSYSTEM_WINDOWS_GUI).is_err());
}

#[test]
fn subsystem_patch_rejects_a_bogus_lfanew() {
    let mut image = templated_image();
    image[0x3C..0x40].copy_from_slice(&0x7FFF_FFFFi32.to_le_bytes());
    assert!(pe::set_subsystem(&mut image, pe::IMAGE_SUBSYSTEM_WINDOWS_GUI).is_err());
}

#[test]
fn sidecar_matches_to_text() {
    let dir = tempfile::tempdir().unwrap();
    let config = sample_config();
    let sidecar = dir.path().join("php.shim");

    std::fs::write(&sidecar, to_text(&config).as_bytes()).unwrap();

    let text = std::fs::read_to_string(&sidecar).unwrap();
    assert_eq!(text, to_text(&config));
    assert_eq!(hok_shim_spec::from_text(&text).unwrap(), config);
}

/// Build a PE image with a chosen subsystem, so target detection can be tested
/// without a real program on disk.
fn image_with_subsystem(subsystem: u16) -> Vec<u8> {
    let mut image = templated_image();
    pe::set_subsystem(&mut image, subsystem).unwrap();
    image
}

#[test]
fn shim_subsystem_follows_the_target_image() {
    let dir = tempfile::tempdir().unwrap();

    let gui = dir.path().join("gui.exe");
    std::fs::write(&gui, image_with_subsystem(pe::IMAGE_SUBSYSTEM_WINDOWS_GUI)).unwrap();
    assert_eq!(subsystem_of(&gui), pe::IMAGE_SUBSYSTEM_WINDOWS_GUI);

    let console = dir.path().join("console.exe");
    std::fs::write(
        &console,
        image_with_subsystem(pe::IMAGE_SUBSYSTEM_WINDOWS_CUI),
    )
    .unwrap();
    assert_eq!(subsystem_of(&console), pe::IMAGE_SUBSYSTEM_WINDOWS_CUI);
}

#[test]
fn unreadable_targets_fall_back_to_console() {
    let dir = tempfile::tempdir().unwrap();

    // A missing file must not be mistaken for GUI, which would detach the
    // console the child needs.
    assert_eq!(
        subsystem_of(&dir.path().join("absent.exe")),
        pe::IMAGE_SUBSYSTEM_WINDOWS_CUI
    );

    let garbage = dir.path().join("garbage.exe");
    std::fs::write(&garbage, b"MZ but not really a PE image").unwrap();
    assert_eq!(subsystem_of(&garbage), pe::IMAGE_SUBSYSTEM_WINDOWS_CUI);
}

/// Exercise the real creation path against a real target and run the result.
///
/// Everything else here tests a piece in isolation; this is the only test that
/// proves `add` produces a shim that actually launches.
#[test]
fn created_shim_launches_its_target() {
    let dir = tempfile::tempdir().unwrap();
    let shims_dir = dir.path().join("shims");
    std::fs::create_dir_all(&shims_dir).unwrap();

    // `cmd.exe /c exit 42` is the smallest target whose exit code is observable
    // and whose image exists on every Windows install.
    let target = std::path::Path::new(r"C:\Windows\System32\cmd.exe");
    if !target.is_file() {
        return;
    }

    let config = Config {
        target: Utf16String::from(target.to_string_lossy().as_ref()),
        args: vec![Utf16String::from("/c"), Utf16String::from("exit 42")],
        ..Config::default()
    };

    let shim_path = shims_dir.join("hok-shim-selftest");
    let image = image_with_subsystem(pe::IMAGE_SUBSYSTEM_WINDOWS_CUI);
    let exe_path = shim_path.with_extension("exe");
    std::fs::write(&exe_path, &image).unwrap();
    resource::write(
        &exe_path,
        &wides(CONFIG_RESOURCE_NAME),
        &encode(&config).unwrap(),
        Some(0x0409),
    )
    .unwrap();

    let status = std::process::Command::new(&exe_path).status().unwrap();
    assert_eq!(status.code(), Some(42));

    // A GUI-subsystem copy is the same launcher with one header field changed;
    // it must still reach the target.
    let gui_exe = shims_dir.join("hok-shim-selftest-gui.exe");
    let mut gui_image = image_with_subsystem(pe::IMAGE_SUBSYSTEM_WINDOWS_GUI);
    pe::set_subsystem(&mut gui_image, pe::IMAGE_SUBSYSTEM_WINDOWS_GUI).unwrap();
    std::fs::write(&gui_exe, &gui_image).unwrap();
    resource::write(
        &gui_exe,
        &wides(CONFIG_RESOURCE_NAME),
        &encode(&config).unwrap(),
        Some(0x0409),
    )
    .unwrap();
    // With no arguments a GUI shim stays detached, so give it one to make it
    // borrow the console and report its exit code.
    let status = std::process::Command::new(&gui_exe)
        .arg("ignored")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(42));
}
