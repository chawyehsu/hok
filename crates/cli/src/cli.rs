use clap::{crate_description, crate_version, App, AppSettings, Arg, SubCommand};

pub fn build() -> App<'static, 'static> {
    let app = App::new("scoop")
        .usage("scoop <command> [<args>]")
        .version(crate_version!())
        .about(crate_description!())
        .after_help("Type 'scoop help <command>' to get help for a specific command.")
        .global_setting(AppSettings::UnifiedHelpMessage)
        .global_setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("bucket")
                .about("Manage manifest buckets")
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
                .subcommand(SubCommand::with_name("list").about("List all added buckets").alias("ls"))
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
                        .alias("ls")
                        .arg(
                            Arg::with_name("query")
                                .help("List caches matching the query")
                                .index(1)
                                .required(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Remove download caches")
                        .alias("rm")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("query").help("Remove caches matching the query"))
                        .arg(
                            Arg::with_name("all")
                                .help("Remove all caches")
                                .short("a")
                                .long("all")
                                .conflicts_with("query"),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("cat")
                .about("Display manifest content of a package")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("package")
                        .help("Name of the package to be inspected")
                        .required(true),
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
                .about("Configuration manipulations")
                .setting(AppSettings::ArgRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("edit").about("Open the config file with the editor"),
                )
                .subcommand(SubCommand::with_name("list").about("List all settings in key-value"))
                .subcommand(
                    SubCommand::with_name("set")
                        .about("Add a new setting to the config file")
                        .arg(
                            Arg::with_name("key")
                                .help("The key of the config")
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("value")
                                .help("The value of the setting")
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("unset")
                        .about("Remove a setting from config file")
                        .arg(
                            Arg::with_name("key")
                                .help("The key of the setting")
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("hold")
                .about("Hold package(s) to disable updates")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("package")
                        .help("The package(s) to be held")
                        .required(true)
                        .multiple(true)
                ),
        )
        .subcommand(
            SubCommand::with_name("home")
                .about("Open the homepage of given package")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("package")
                        .help("The package name")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("info")
                .about("Display information about a package")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("package")
                        .help("Name of the package to be inspected")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("install")
                .about("Install package(s)")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("package")
                        .help("The package(s) to be installed")
                        .required(true)
                        .multiple(true),
                )
                // .arg(
                //     Arg::with_name("independent")
                //         .help("Do not automatically install dependencies")
                //         .short("i")
                //         .long("independent"),
                // )
                .arg(
                    Arg::with_name("download-only")
                        .help("Download packages without performing installation")
                        .short("D")
                        .long("download-only"),
                )
                .arg(
                    Arg::with_name("ignore-broken")
                        .long_help(
                            "Ignore broken packages while performing installation.\n\
                            By default, install will be interrupted when a package \
                            fails during the install workflow, including download \
                            errors, hash mismatch, scripting errors. Turning this \
                            option on will ignore failures and ensure a complete install transaction.",
                        )
                        .short("e")
                        .long("ignore-broken"),
                )
                .arg(
                    Arg::with_name("no-cache")
                        .help("Perform a fresh download despite local caches")
                        .short("c")
                        .long("no-cache"),
                )
                .arg(
                    Arg::with_name("no-hash-check")
                        .help("Skip package integrity check, USE WITH CAUTION!")
                        .long("no-hash-check"),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("List installed package(s)")
                .arg(
                    Arg::with_name("package")
                        .help("Specified package(s) to be listed")
                        .required(false)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("upgradable")
                        .help("List upgradable package(s)")
                        .short("u")
                        .long("upgradable"),
                ),
        )
        .subcommand(
            SubCommand::with_name("search")
                .about("Search available package(s)")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("query")
                        .help("The query string")
                        .required(true)
                        .multiple(true),
                )
                .arg(
                    Arg::with_name("explicit")
                        .help("Turn off fuzzy search and use explicit search through package names")
                        .long("explicit")
                        .conflicts_with_all(&["names-only", "with-binaries"]),
                )
                .arg(
                    Arg::with_name("names-only")
                        .help("Only search through package names")
                        .short("n")
                        .long("names-only")
                        .conflicts_with_all(&["with-binaries"]),
                )
                .arg(
                    Arg::with_name("with-binaries")
                        .help("Also search through package binaries")
                        .long("--with-binaries"),
                ),
        )
        .subcommand(
            SubCommand::with_name("unhold")
                .about("Unhold package(s) to enable updates")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg(
                    Arg::with_name("package")
                        .help("The package(s) to be unheld")
                        .required(true)
                        .multiple(true)
                ),
        )
        .subcommand(SubCommand::with_name("update").about("Fetch and update all buckets"))
        .subcommand(
            SubCommand::with_name("upgrade")
                .about("Upgrade installed package(s)")
                .arg(
                    Arg::with_name("package")
                        .help("Specified package(s) to be upgraded")
                        .required(false)
                        .multiple(true),
                ),
        );

    app
}
