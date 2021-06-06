use anyhow::Result;
use futures::{executor::block_on, future::join_all};
use crate::{
    bucket::Bucket,
    fs::leaf_base,
    manifest::{BinType, Manifest, StringOrStringArray},
    Scoop,
};

struct SearchMatch {
    name: String,
    version: String,
    bin: Option<String>,
}

struct Matches {
    bucket: String,
    collected: Vec<SearchMatch>,
}

async fn walk_manifests(
    bucket_name: &str,
    bucket: &Bucket,
    query: &str,
    with_binary: bool,
) -> Result<Matches> {
    let mut search_matches: Vec<SearchMatch> = Vec::new();

    for app in bucket.available_manifests()?.iter() {
        // trace!("Searching manifest {}", app.display());

        let app_name = leaf_base(app);
        // substring check on app_name
        if app_name.contains(query) {
            let manifest = Manifest::from_path(app);
            if manifest.is_err() {
                trace!("{:?}", manifest.err());
                continue;
            }

            let version = manifest.unwrap().data.version;
            search_matches.push(SearchMatch {
                name: app_name,
                version,
                bin: None,
            });
        } else {
            // Searching binaries requires a very-high overhead (reading all json files),
            // will not do binary search without the option.
            if !with_binary {
                continue;
            }

            let manifest = Manifest::from_path(app);
            if manifest.is_err() {
                trace!("{:?}", manifest.err());
                continue;
            }

            let Manifest {
                name,
                path: _,
                bucket: _,
                data,
            } = manifest.unwrap();

            let mut bin_matches = Vec::new();
            match data.bin {
                None => continue,
                Some(bintype) => match bintype {
                    BinType::Single(bin) => {
                        if bin.contains(query) {
                            bin_matches.push(bin);
                        }
                    }
                    BinType::Multiple(bins) => {
                        bins.into_iter().for_each(|bin| {
                            if bin.contains(query) {
                                bin_matches.push(bin);
                            }
                        });
                    }
                    BinType::Complex(complex) => {
                        complex.into_iter().for_each(|item| match item {
                            StringOrStringArray::String(bin) => {
                                if bin.contains(query) {
                                    bin_matches.push(bin);
                                }
                            }
                            StringOrStringArray::Array(pair) => {
                                if pair[1].contains(query) {
                                    bin_matches.push(pair[1].to_string());
                                }
                            }
                        });
                    }
                },
            }

            if bin_matches.len() > 0 {
                let version = data.version;
                let bin = format!("'{}'", bin_matches[0].to_string());
                search_matches.push(SearchMatch {
                    name,
                    version,
                    bin: Some(bin),
                });
            }
        }
    }

    Ok(Matches {
        bucket: bucket_name.to_string(),
        collected: search_matches,
    })
}

impl<'a> Scoop<'a> {
    pub fn search(&mut self, query: &str, with_binary: bool) -> Result<()> {
        // Load all local buckets
        let buckets = self.bucket_manager.get_buckets();

        let mut matches: Vec<Matches> = Vec::new();
        let mut futures = Vec::new();

        for (bucket_name, bucket) in buckets {
            futures.push(walk_manifests(bucket_name, bucket, query, with_binary));
        }

        block_on(async {
            let all_matches = join_all(futures).await;
            for m in all_matches {
                matches.push(m.unwrap());
            }
        });

        matches.sort_by_key(|k| k.bucket.to_string());

        for m in matches {
            if m.collected.len() > 0 {
                println!("'{}' bucket:", m.bucket);
                for sm in m.collected {
                    if sm.bin.is_none() {
                        println!("  {} ({})", sm.name, sm.version);
                    } else {
                        println!(
                            "  {} ({}) --> includes {}",
                            sm.name,
                            sm.version,
                            sm.bin.unwrap()
                        );
                    }
                }
                println!("");
            }
        }

        Ok(())
    }
}
