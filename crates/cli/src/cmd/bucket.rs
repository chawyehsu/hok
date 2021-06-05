use clap::ArgMatches;

use scoop_core::bucket;
use scoop_core::Scoop;

pub fn cmd_bucket(matches: &ArgMatches, scoop: &mut Scoop) {
    match matches.subcommand() {
        ("add", Some(matches)) => {
            let bucket_name = matches.value_of("name").unwrap();

            if scoop.bucket_manager.contains(bucket_name) {
                println!("The '{}' already exists.", bucket_name);
                std::process::exit(1);
            }

            if bucket::is_known_bucket(bucket_name) {
                let bucket_url = bucket::known_bucket_url(bucket_name).unwrap();
                scoop.git.clone(bucket_name, bucket_url).unwrap();
            } else {
                match matches.value_of("repo") {
                    Some(repo) => {
                        scoop.git.clone(bucket_name, repo).unwrap();
                    }
                    None => {
                        eprintln!("<repo> is required for unknown bucket.");
                        std::process::exit(1);
                    }
                }
            }
        }
        ("list", Some(_)) => {
            for b in scoop.bucket_manager.get_buckets() {
                println!("{}", b.0.as_str());
            }
        }
        ("known", Some(_)) => {
            for b in bucket::known_buckets() {
                println!("{}", b);
            }
        }
        ("rm", Some(matches)) => {
            let bucket_name = matches.value_of("name").unwrap();

            if scoop.bucket_manager.contains(bucket_name) {
                let bucket_dir = scoop.dir("buckets").join(bucket_name);
                if bucket_dir.exists() {
                    match remove_dir_all::remove_dir_all(bucket_dir) {
                        Ok(()) => {}
                        Err(e) => panic!("failed to remove '{}' bucket. {}", bucket_name, e),
                    };
                }
            } else {
                println!("The '{}' bucket not found.", bucket_name);
            }
        }
        _ => unreachable!(),
    }
}
