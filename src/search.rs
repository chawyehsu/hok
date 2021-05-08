use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use crate::Scoop;
use regex::RegexBuilder;

struct SearchMatch {
  name: String,
  version: String,
  bin: Option<String>
}

impl Scoop {
  pub fn search(&self, query: &str, with_binary: bool) -> Result<()> {
    let re = RegexBuilder::new(query)
      .case_insensitive(true).build()?;
    let buckets = self.get_local_buckets_name()?;

    for bucket in buckets {
      let mut matches: Vec<SearchMatch> = Vec::new();

      if let Some(apps) = self.apps_in_bucket(bucket.as_str())? {
        for app in apps.iter() {
          let app_name = app.file_name();
          let app_name = app_name.to_str().unwrap().trim_end_matches(".json");

          if re.is_match(app_name) {
            let manifest = self.manifest_from_local(app.path());
            if manifest.is_err() { continue; }
            let manifest = manifest?;
            let version = manifest.get("version");

            // filter bad manifest doesn't contain `version`
            if version.is_none() {
              continue;
            }

            let name = app_name.to_string();
            let version = version.unwrap().as_str().unwrap().to_owned();
            let bin: Option<String> = None;

            let sm = SearchMatch {
              name,
              version,
              bin
            };

            matches.push(sm);
          } else {
            // Searching binaries requires a very-high overhead (reading all json files),
            // will not do binary search without the option.
            if !with_binary { continue; }

            let manifest = self.manifest_from_local(app.path());
            if manifest.is_err() { continue; }
            let manifest = manifest?;
            let bin = manifest.get("bin");

            // filter manifest doesn't contain `bin`
            if bin.is_none() {
              continue;
            }

            let bin = bin.unwrap();
            let match_bin: Option<Vec<String>> = match bin {
              Value::String(bin) => {
                let bin = Path::new(bin)
                  .file_name().unwrap().to_str().unwrap();

                if re.is_match(bin) {
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

                      if re.is_match(bin) {
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

                          if re.is_match(bin) {
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
                          if re.is_match(bin) {
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
                let version = manifest.get("version");
                let name = app_name.to_string();
                let version = version.unwrap().as_str().unwrap().to_owned();
                let bin: Option<String> = Some(bins.get(0).unwrap().to_owned());

                let sm = SearchMatch {
                  name,
                  version,
                  bin
                };

                matches.push(sm);
              },
              None => {
                continue;
              }
            }
          }
        }
      }

      if matches.len() > 0 {
        println!("'{}' bucket:", bucket);
        for sm in matches {
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
