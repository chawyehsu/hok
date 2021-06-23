use clap::ArgMatches;
use scoop_core::{BinType, License, Scoop, StringOrStringArray};

pub fn cmd_info(matches: &ArgMatches, scoop: &mut Scoop) {
    let app = matches.value_of("app").unwrap();
    match scoop.find_local_manifest(app) {
        Ok(Some(manifest)) => {
            let name = manifest.name;
            let path = manifest.path;
            let bucket = manifest.bucket;
            let data = manifest.data;

            // Name
            println!("Name: {}", name);
            // Bucket
            if bucket.is_some() {
                println!("Bucket: {}", bucket.unwrap());
            }
            // Description
            if data.description.is_some() {
                println!("Description: {}", data.description.unwrap());
            }
            // Version
            println!("Version: {}", data.version);
            // Homepage
            if data.homepage.is_some() {
                println!("Website: {}", data.homepage.unwrap());
            }
            // License
            if data.license.is_some() {
                let licenses = data.license.unwrap();

                match licenses {
                    License::Simple(str) => println!("License: {}", str),
                    License::Complex(pair) => {
                        println!("License:");
                        match pair.url {
                            Some(url) => println!("  {} ({})", pair.identifier, url),
                            None => println!("  {}", pair.identifier),
                        }
                    }
                }
            }
            // Manifest
            println!("Manifest:\n  {}", path.display());

            // Binaries
            match data.bin {
                Some(bintype) => match bintype {
                    BinType::String(bin) => println!("Binary: {}", bin),
                    BinType::Array(complex) => {
                        println!("Binaries:");

                        let mut bins = Vec::new();

                        for item in complex.into_iter() {
                            match item {
                                StringOrStringArray::String(bin) => bins.push(bin),
                                StringOrStringArray::Array(pair) => bins.push(pair[1].to_string()),
                            }
                        }

                        println!("  {}", bins.join(" "));
                    }
                },
                None => {}
            }

            std::process::exit(0);
        }
        Ok(None) => {
            eprintln!("Could not find manifest for '{}'", app);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to operate. ({})", e);
            std::process::exit(1);
        }
    }
}
