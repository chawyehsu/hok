use clap::{App, Arg, AppSettings, SubCommand, crate_name, crate_version, crate_description};

pub fn build_app() -> App<'static, 'static> {
  let app = App::new(crate_name!())
    .version(crate_version!())
    .about(crate_description!())
    .after_help("Type 'scoop help <command>' to get help for a specific command.")
    .global_setting(AppSettings::VersionlessSubcommands)
    .setting(AppSettings::ArgRequiredElseHelp)
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
              .required(true)
          )
          .arg(
            Arg::with_name("repo")
              .help("The bucket repository url")
              .index(2)
          )
        )
        .subcommand(
          SubCommand::with_name("list")
            .about("List all added buckets")
        )
        .subcommand(
          SubCommand::with_name("known")
            .about("List known buckets")
        )
        .subcommand(
          SubCommand::with_name("rm")
            .about("Remove a bucket")
            .alias("remove")
            .arg(
              Arg::with_name("name")
                .help("The bucket name")
                .required(true)
            )
        )
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
              .required(false)
          )
        )
        .subcommand(
          SubCommand::with_name("rm")
          .about("Remove the download cache")
          .alias("remove")
          .setting(AppSettings::ArgRequiredElseHelp)
          .arg(
            Arg::with_name("app")
              .help("The app name")
          )
          .arg(
            Arg::with_name("all")
              .help("Remove all download caches")
              .short("a")
              .long("all")
              .conflicts_with("app")
          )
        )
    )
    // .subcommand(
    //   SubCommand::with_name("cleanup")
    //     .about("Cleanup apps by removing old versions")
    // )
    .subcommand(
      SubCommand::with_name("config")
        .about("Get or set configuration values")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::ArgsNegateSubcommands)
        .subcommand(
          SubCommand::with_name("list")
          .about("List all configurations")
          .alias("ls")
        )
        .subcommand(
          SubCommand::with_name("remove")
          .about("Remove a configuration value")
          .alias("rm")
          .setting(AppSettings::ArgRequiredElseHelp)
          .arg(
            Arg::with_name("name")
              .help("The name of a configuration")
          )
        )
        .arg(
          Arg::with_name("name")
            .help("The name of a configuration")
        )
        .arg(
          Arg::with_name("value")
            .help("The value of a configuration")
        )
    )
    .subcommand(
      SubCommand::with_name("home")
        .about("Opens the app homepage")
        .arg(
          Arg::with_name("app")
            .help("The app name")
            .required(true)
        )
    )
    .subcommand(
      SubCommand::with_name("search")
        .about("Searches for apps that are available to install")
        .arg(
          Arg::with_name("query")
            .help("The query string (regex is support)")
            .required(true)
        )
    )
    .subcommand(
      SubCommand::with_name("update")
        .about("Fetch and update all buckets")
    // )
    // .subcommand(
    //   SubCommand::with_name("info")
    //     .about("Display information about an app")
    // )
    // .subcommand(
    //   SubCommand::with_name("list")
    //     .about("List installed apps")
    );

  app
}
