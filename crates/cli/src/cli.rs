use clap::{crate_description, crate_name, crate_version, App, AppSettings, Arg, SubCommand};

pub fn build_app() -> App<'static, 'static> {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .after_help("Type 'scoop help <command>' to get help for a specific command.")
        .global_setting(AppSettings::UnifiedHelpMessage)
        .global_setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("bucket")
                .about("Manage Scoop buckets")
                .setting(AppSettings::ArgRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Add a bucket")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(
                            Arg::with_name("name")
                                .help("The bucket name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("repo")
                                .help("The bucket repository url")
                                .index(2)
                                .required(false),
                        ),
                )
                .subcommand(SubCommand::with_name("list").about("List all added buckets"))
                .subcommand(SubCommand::with_name("known").about("List known buckets"))
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Remove a bucket")
                        .alias("rm")
                        .arg(
                            Arg::with_name("name")
                                .help("The bucket name")
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("cache")
                .about("List or remove download caches")
                .setting(AppSettings::ArgRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("list")
                        .about("List download caches")
                        .arg(
                            Arg::with_name("app")
                                .help("The app name")
                                .index(1)
                                .required(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Remove the download cache")
                        .alias("rm")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("app").help("The app name"))
                        .arg(
                            Arg::with_name("all")
                                .help("Remove all download caches")
                                .short("a")
                                .long("all")
                                .conflicts_with("app"),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("cleanup")
                .about("Cleanup apps by removing old versions")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("app")
                        .help("Given app name to be cleaned up")
                        .conflicts_with("all"),
                )
                .arg(
                    Arg::with_name("all")
                        .help("Cleanup all apps")
                        .short("a")
                        .long("all")
                        .conflicts_with("app"),
                )
                .arg(
                    Arg::with_name("cache")
                        .help("Remove outdated download cache")
                        .short("k")
                        .long("cache"),
                ),
        )
        .subcommand(
            SubCommand::with_name("config")
                .about("Get or set configuration values")
                .setting(AppSettings::ArgRequiredElseHelp)
                .setting(AppSettings::ArgsNegateSubcommands)
                .arg(
                    Arg::with_name("edit")
                        .help("Open an editor to modify the config file")
                        .short("e")
                        .long("edit")
                        .conflicts_with_all(&["list", "set", "unset"]),
                )
                .arg(
                    Arg::with_name("list")
                        .help("List all key-value sets in config file")
                        .short("l")
                        .long("list")
                        .conflicts_with_all(&["edit", "set", "unset"]),
                )
                .arg(
                    Arg::with_name("set")
                        .help("Add a new variable to the config file")
                        .long("set")
                        .value_names(&["key", "value"])
                        .conflicts_with_all(&["edit", "list", "unset"]),
                )
                .arg(
                    Arg::with_name("unset")
                        .help("Remove a variable matching the key from config file")
                        .long("unset")
                        .value_name("key")
                        .conflicts_with_all(&["edit", "list", "set"]),
                ),
        )
        .subcommand(
            SubCommand::with_name("hold")
                .about("Hold an app to disable updates")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::with_name("app").help("The app name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("home")
                .about("Opens the app homepage")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::with_name("app").help("The app name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("info")
                .about("Display information about an app")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("app")
                        .help("The app to be inspected")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("install")
                .about("Install apps")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("app")
                        .help("The app to be installed")
                        .required(true)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("independent")
                        .help("Do not automatically install dependencies")
                        .short("i")
                        .long("independent"),
                )
                .arg(
                    Arg::with_name("ignore_cache")
                        .help("Do not use previous download cache")
                        .short("k")
                        .long("ignore-cache"),
                )
                .arg(
                    Arg::with_name("skip_hash_validation")
                        .help("Skip hash validation (use with caution!)")
                        .long("skip-hash-validation"),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("List installed apps"))
        .subcommand(
            SubCommand::with_name("search")
                .about("Searches for apps that are available to install")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("query")
                        .help("The query string")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("status").about("Show status and check for new app versions"),
        )
        .subcommand(
            SubCommand::with_name("unhold")
                .about("Unhold an app to enable updates")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(Arg::with_name("app").help("The app name").required(true)),
        )
        .subcommand(SubCommand::with_name("update").about("Fetch and update all buckets"));

    app
}
