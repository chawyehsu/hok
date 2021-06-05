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
                                .index(2),
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
                .about("Show or clear the download cache")
                .subcommand(
                    SubCommand::with_name("show")
                        .about("Show the download cache")
                        .arg(
                            Arg::with_name("app")
                                .help("The app name")
                                .index(1)
                                .required(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("rm")
                        .about("Remove the download cache")
                        .alias("remove")
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
                .arg(Arg::with_name("app").help("The app name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("home")
                .about("Opens the app homepage")
                .arg(Arg::with_name("app").help("The app name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Searches for apps that are available to install")
                .arg(
                    Arg::with_name("query")
                        .help("The query string, precision searching by default")
                        .required(true),
                )
                .arg(
                    Arg::with_name("binary")
                        .help("Enable search on manifest 'bin' property")
                        .short("b")
                        .long("with-binary"),
                ),
        )
        .subcommand(SubCommand::with_name("update").about("Fetch and update all buckets"))
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
                    Arg::with_name("global")
                        .help("Install the app globally")
                        .short("g")
                        .long("global"),
                )
                .arg(
                    Arg::with_name("nocache")
                        .help("Don't use the download cache")
                        .short("k")
                        .long("no-cache"),
                )
                .arg(
                    Arg::with_name("skip")
                        .help("Skip hash validation (use with caution!)")
                        .short("s")
                        .long("skip"),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("List installed apps"));

    app
}
