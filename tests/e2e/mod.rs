use insta_cmd::assert_cmd_snapshot;

use crate::utils::{TestWorkspace, common_snapshot_settings};

#[test]
fn hok_help() {
    let ws = TestWorkspace::new();
    let settings = common_snapshot_settings!(ws);

    settings.bind(|| {
        assert_cmd_snapshot!("hok_help", ws.app().arg("--help"));
    });
}

#[test]
fn hok_version() {
    let ws = TestWorkspace::new();
    let mut settings = common_snapshot_settings!(ws);

    // Redact the version number from the output to make snapshots stable.
    settings.add_filter(r"hok \d+\.\d+\.\d+([-.\w]+)?", "hok [VERSION]");

    settings.bind(|| {
        assert_cmd_snapshot!("hok_version", ws.app().arg("--version"));
    });
}

#[test]
fn hok_bucket_list_known() {
    let ws = TestWorkspace::new();
    let settings = common_snapshot_settings!(ws);

    settings.bind(|| {
        assert_cmd_snapshot!(
            "hok_bucket_list_known",
            ws.app().arg("bucket").arg("list").arg("--known")
        );
    });
}
