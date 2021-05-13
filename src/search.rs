use std::{path::Path};

use anyhow::Result;
use futures::{executor::block_on, future::join_all};
use serde_json::Value;
use regex::RegexBuilder;
use crate::{Scoop, manifest::ScoopAppManifest};

struct SearchMatch {
  name: String,
  version: String,
  bin: Option<String>
}

struct Matches {
  bucket: String,
  collected: Vec<SearchMatch>
}

impl Scoop {
  async fn walk_manifests<F>(
    &self,
    bucket_name: String,
    with_binary: bool,
    match_helper: F
  ) -> Result<Matches> where F: Fn(String) -> bool {
    let mut search_matches: Vec<SearchMatch> = Vec::new();
    let apps = self.apps_in_local_bucket(&bucket_name)?;

    for app in apps.iter() {
      let app_name = app.file_name();
      let app_name = app_name.to_str().unwrap().trim_end_matches(".json");

      if match_helper(app_name.to_owned()) {
        let manifest = ScoopAppManifest::from_path(app.path());
        if manifest.is_err() { continue; }
        let manifest = manifest?;
        let version = manifest.version;
        let name = app_name.to_string();
        let bin: Option<String> = None;

        let sm = SearchMatch {
          name,
          version,
          bin
        };

        search_matches.push(sm);
      } else {
        // Searching binaries requires a very-high overhead (reading all json files),
        // will not do binary search without the option.
        if !with_binary { continue; }

        let manifest = ScoopAppManifest::from_path(app.path());
        if manifest.is_err() { continue; }
        let manifest = manifest?;
        let bin = manifest.json.get("bin");

        // filter manifest doesn't contain `bin`
        if bin.is_none() {
          continue;
        }

        let bin = bin.unwrap();
        let match_bin: Option<Vec<String>> = match bin {
          Value::String(bin) => {
            let bin = Path::new(bin)
              .file_name().unwrap().to_str().unwrap();

            if match_helper(bin.to_owned()) {
              Some(vec![format!("'{}'", bin.to_string())])
            } else {
              None
            }
          },
          Value::Array(bins) => {
            let mut bin_matches = Vec::new();

            for bin in bins {
              match bin {
                Value::String(bin) => {
                  let bin = Path::new(bin)
                    .file_name().unwrap().to_str().unwrap();

                  if match_helper(bin.to_owned()) {
                    bin_matches.push(format!("'{}'", bin.to_string()));
                    continue;
                  }
                },
                Value::Array(bin_pair) => {
                  // test bin
                  let bin = bin_pair.get(0).unwrap();
                  match bin {
                    Value::String(bin) => {
                      let bin = Path::new(bin)
                        .file_name().unwrap().to_str().unwrap();

                      if match_helper(bin.to_owned()) {
                        bin_matches.push(format!("'{}'", bin.to_string()));
                        continue;
                      }
                    },
                    _ => {}
                  }

                  // test alias
                  let bin = bin_pair.get(1).unwrap();
                  match bin {
                    Value::String(bin) => {
                      if match_helper(bin.to_owned()) {
                        bin_matches.push(format!("'{}'", bin.to_string()));
                        continue;
                      }
                    },
                    _ => {}
                  }
                },
                _ => {}
              }
            }

            if bin_matches.len() > 0 {
              Some(bin_matches)
            } else {
              None
            }
          },
          _ => {
            None
          }
        };

        match match_bin {
          Some(bins) => {
            let version = manifest.json.get("version");
            let name = app_name.to_string();
            let version = version.unwrap().as_str().unwrap().to_owned();
            let bin: Option<String> = Some(bins.get(0).unwrap().to_owned());

            let sm = SearchMatch {
              name,
              version,
              bin
            };

            search_matches.push(sm);
          },
          None => {
            continue;
          }
        }
      }
    }

    Ok(Matches {
      bucket: bucket_name,
      collected: search_matches
    })
  }

  pub fn search(&self, query: &str, fuzzy: bool, with_binary: bool) -> Result<()> {
    let buckets = self.local_buckets()?;
    let re = RegexBuilder::new(query)
      .case_insensitive(true).build()?;
    let match_helper = |input: String| -> bool {
      if fuzzy {
        re.is_match(input.as_str())
      } else {
        input.eq(query)
      }
    };

    let mut matches: Vec<Matches> = Vec::new();
    let mut futures = Vec::new();

    for bucket in buckets {
      futures.push(
        self.walk_manifests(
          bucket.0.to_string(),
          with_binary,
          match_helper)
      );
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
            println!("  {} ({}) --> includes {}", sm.name, sm.version, sm.bin.unwrap());
          }
        }
        println!("");
      }
    }

    Ok(())
  }
}
