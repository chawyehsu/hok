use clap::{crate_description, crate_name, crate_version, Arg, ArgAction, Command};

pub fn build() -> Command {
    let app = Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .after_help(format!(
            "Type '{} help <command>' to get help for a specific command.",
            crate_name!()
        ))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .max_term_width(100)
        .subcommand(
            Command::new("bucket")
                .about("Manage manifest buckets")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("add")
                        .about("Add a bucket")
                        .arg_required_else_help(true)
                        .arg(
                            Arg::new("name")
                                .help("The bucket name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::new("repo")
                                .help("The bucket repository url (optional for known buckets)")
                                .index(2),
                        ),
                )
                .subcommand(
                    Command::new("list").about("List buckets").alias("ls").arg(
                        Arg::new("known")
                            .help("List known buckets")
                            .short('k')
                            .action(ArgAction::SetTrue),
                    ),
                )
                .subcommand(
                    Command::new("remove")
                        .about("Remove bucket(s)")
                        .alias("rm")
                        .arg_required_else_help(true)
                        .arg(
                            Arg::new("name")
                                .help("The bucket name")
                                .required(true)
                                .action(ArgAction::Append),
                        ),
                ),
        )
        .subcommand(
            Command::new("cache")
                .about("List or remove download caches")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("list")
                        .about("List download caches")
                        .alias("ls")
                        .arg(
                            Arg::new("query")
                                .help("List caches matching the query")
                                .index(1),
                        ),
                )
                .subcommand(
                    Command::new("remove")
                        .about("Remove download caches")
                        .alias("rm")
                        .arg_required_else_help(true)
                        .arg(Arg::new("query").help("Remove caches matching the query"))
                        .arg(
                            Arg::new("all")
                                .help("Remove all caches")
                                .short('a')
                                .long("all")
                                .action(ArgAction::SetTrue)
                                .conflicts_with("query"),
                        ),
                ),
        )
        .subcommand(
            Command::new("cat")
                .about("Display manifest content of a package")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("Name of the package to be inspected")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("cleanup")
                .about("Cleanup apps by removing old versions")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("app")
                        .help("Given named app(s) to be cleaned up")
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("cache")
                        .help("Remove download cache simultaneously")
                        .short('k')
                        .long("cache")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("config")
                .about("Configuration manipulations")
                .arg_required_else_help(true)
                .subcommand(
                    Command::new("edit").about("Edit the config file [default editor: notepad]"),
                )
                .subcommand(
                    Command::new("list")
                        .alias("ls")
                        .about("List all settings in key-value"),
                )
                .subcommand(
                    Command::new("set")
                        .about("Add a new setting to the config file")
                        .arg_required_else_help(true)
                        .arg(Arg::new("key").help("The key of the config").required(true))
                        .arg(
                            Arg::new("value")
                                .help("The value of the setting")
                                .required(true),
                        ),
                )
                .subcommand(
                    Command::new("unset")
                        .about("Remove a setting from config file")
                        .arg_required_else_help(true)
                        .arg(
                            Arg::new("key")
                                .help("The key of the setting")
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            Command::new("hold")
                .about("Hold package(s) to disable updates")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("The package(s) to be held")
                        .required(true)
                        .action(ArgAction::Append),
                ),
        )
        .subcommand(
            Command::new("home")
                .about("Open the homepage of given package")
                .arg_required_else_help(true)
                .arg(Arg::new("package").help("The package name").required(true)),
        )
        .subcommand(
            Command::new("info")
                .about("Display information about a package")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("Name of the package to be inspected")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("install")
                .about("Install package(s)")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("The package(s) to be installed")
                        .required(true)
                        .action(ArgAction::Append),
                )
                // .arg(
                //     Arg::new("independent")
                //         .help("Do not automatically install dependencies")
                //         .short("i")
                //         .long("independent")
                // )
                .arg(
                    Arg::new("download-only")
                        .help("Download packages without performing installation")
                        .short('D')
                        .long("download-only")
                        .action(ArgAction::SetTrue),
                ), // .arg(
                   //     Arg::new("ignore-broken")
                   //         .long_help(
                   //             "Ignore broken packages while performing installation.\n\
                   //             By default, install will be interrupted when a package \
                   //             fails during the install workflow, including download \
                   //             errors, hash mismatch, scripting errors. Turning this \
                   //             option on will ignore failures and ensure a complete install transaction.",
                   //         )
                   //         .short('e')
                   //         .long("ignore-broken")
                   //         .action(ArgAction::SetTrue)
                   // )
                   // .arg(
                   //     Arg::new("ignore-cache")
                   //         .help("Perform a fresh download despite local caches")
                   //         .short('F')
                   //         .long("ignore-cache")
                   //         .action(ArgAction::SetTrue)
                   // )
                   // .arg(
                   //     Arg::new("no-hash-check")
                   //         .help("Skip package integrity check, USE WITH CAUTION!")
                   //         .long("no-hash-check")
                   //         .action(ArgAction::SetTrue)
                   // )
        )
        .subcommand(
            Command::new("list")
                .about("List installed package(s)")
                .arg(Arg::new("package").help(
                    "Specify package(s) to be listed, bucket prefix can be used to narrow results",
                ))
                .arg(
                    Arg::new("upgradable")
                        .help("List upgradable package(s)")
                        .short('u')
                        .long("upgradable")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("held")
                        .help("List held package(s)")
                        .short('H')
                        .long("held")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("search")
                .about("Search available package(s)")
                .long_about(
                    "Search available package(s) from synced buckets.\n\
                    The query is performed against package names by default, \
                    use --with-description or --with-binary to search through \
                    package descriptions or binaries.",
                )
                .arg_required_else_help(true)
                .arg(
                    Arg::new("query")
                        .help("The query string")
                        .required(true)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("with-binary")
                        .help("Search through package binaries as well")
                        .short('B')
                        .long("with-binary")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("with-description")
                        .help("Search through package descriptions as well")
                        .short('D')
                        .long("with-description")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("unhold")
                .about("Unhold package(s) to enable updates")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("The package(s) to be unheld")
                        .required(true)
                        .action(ArgAction::Append),
                ),
        )
        .subcommand(Command::new("update").about("Fetch and update all buckets"))
        .subcommand(
            Command::new("upgrade")
                .about("Upgrade installed package(s)")
                .arg(Arg::new("package").help("Specified package(s) to be upgraded")),
        );

    app
}
