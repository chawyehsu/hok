use clap::{App, Arg, SubCommand, crate_name, crate_version, crate_description};

pub fn build_app() -> App<'static, 'static> {
  let app = App::new(crate_name!())
    .version(crate_version!())
    .about(crate_description!())
    .after_help("Type 'scoop help <command>' to get help for a specific command.")
    .subcommand(
      SubCommand::with_name("alias")
        .about("Manage scoop aliases")
    )
    .subcommand(
      SubCommand::with_name("bucket")
        .about("Manage Scoop buckets")
        .subcommand(
          SubCommand::with_name("add")
          .about("Add a bucket")
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
            )
        )
    )
    .subcommand(
      SubCommand::with_name("cache")
        .about("Show or clear the download cache")
    )
    .subcommand(
      SubCommand::with_name("cleanup")
        .about("Cleanup apps by removing old versions")
    )
    .subcommand(
      SubCommand::with_name("config")
        .about("Get or set configuration values")
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
      SubCommand::with_name("list")
        .about("Get or set configuration values")
    );

  app
}
