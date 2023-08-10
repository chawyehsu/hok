use clap::{crate_description, crate_name, crate_version, Arg, ArgAction, Command};

pub fn build() -> Command {
    Command::new(crate_name!())
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
                            .long("known")
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
                .about("Package cache management")
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
                .about("Inspect the manifest of a package")
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
                .about("Configuration management")
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
                .about("Hold package(s) to disable changes")
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
                .about("Browse the homepage of a package")
                .arg_required_else_help(true)
                .arg(Arg::new("package").help("The package name").required(true)),
        )
        .subcommand(
            Command::new("info")
                .about("Show package(s) basic information")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("query")
                        .help("The query string (regex supported)")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("install")
                .about("Install package(s)")
                .alias("i")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("The package(s) to install")
                        .required(true)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("download-only")
                        .help("Download package(s) without performing installation")
                        .short('d')
                        .long("download-only")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("ignore-failure")
                        .help("Ignore failures to ensure a complete transaction")
                        .short('f')
                        .long("ignore-failure")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("offline")
                        .help("Leverage cache and suppress network access")
                        .short('o')
                        .long("offline")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("assume-yes")
                        .help("Assume yes to all prompts and run non-interactively")
                        .short('y')
                        .long("assume-yes")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("ignore-cache")
                        .help("Ignore cache and force download")
                        .short('D')
                        .long("ignore-cache")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("independent")
                        .help("Do not install dependencies (may break packages)")
                        .short('I')
                        .long("independent")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-replace")
                        .help("Do not replace package(s)")
                        .short('R')
                        .long("no-replace")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("escape-hold")
                        .help("Escape hold to allow changes on held package(s)")
                        .short('S')
                        .long("escape-hold")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-upgrade")
                        .help("Do not upgrade package(s)")
                        .short('U')
                        .long("no-upgrade")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-hash-check")
                        .help("Skip package integrity check")
                        .long("no-hash-check")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("list")
                .about("List installed package(s)")
                .arg(
                    Arg::new("query")
                        .help("The query string (regex supported by default)")
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("explicit")
                        .help("Turn regex off and use explicit matching")
                        .short('e')
                        .long("explicit")
                        .requires("query")
                        .action(ArgAction::SetTrue),
                )
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
                        .help("The query string (regex supported by default)")
                        .required(true)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("explicit")
                        .help("Turn regex off and use explicit matching")
                        .short('e')
                        .long("explicit")
                        .action(ArgAction::SetTrue)
                        .conflicts_with_all(["with-description", "with-binary"]),
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
                .about("Unhold package(s) to enable changes")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("The package(s) to be unheld")
                        .required(true)
                        .action(ArgAction::Append),
                ),
        )
        .subcommand(
            Command::new("uninstall")
                .about("Uninstall package(s)")
                .alias("rm")
                .alias("remove")
                .arg_required_else_help(true)
                .arg(
                    Arg::new("package")
                        .help("The package(s) to uninstall")
                        .required(true)
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("cascade")
                        .help("Remove unneeded dependencies as well")
                        .short('c')
                        .long("cascade")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("purge")
                        .help("Purge package(s) persistent data as well")
                        .short('p')
                        .long("purge")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("assume-yes")
                        .help("Assume yes to all prompts and run non-interactively")
                        .short('y')
                        .long("assume-yes")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-dependent-check")
                        .help("Disable dependent check (may break other packages)")
                        .long("no-dependent-check")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("escape-hold")
                        .help("Escape hold to allow to uninstall held package(s)")
                        .short('S')
                        .long("escape-hold")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("update")
                .about("Fetch and update subscribed buckets")
                .alias("u"),
        )
        .subcommand(
            Command::new("upgrade")
                .about("Upgrade installed package(s)")
                .arg(
                    Arg::new("package")
                        .help("The package(s) to be upgraded (default: all except held)"),
                )
                .arg(
                    Arg::new("ignore-failure")
                        .help("Ignore failures to ensure a complete transaction")
                        .short('f')
                        .long("ignore-failure")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("offline")
                        .help("Leverage cache and suppress network access")
                        .short('o')
                        .long("offline")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("assume-yes")
                        .help("Assume yes to all prompts and run non-interactively")
                        .short('y')
                        .long("assume-yes")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("escape-hold")
                        .help("Escape hold to allow to upgrade held package(s)")
                        .short('S')
                        .long("escape-hold")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("no-hash-check")
                        .help("Skip package integrity check")
                        .long("no-hash-check")
                        .action(ArgAction::SetTrue),
                ),
        )
}
