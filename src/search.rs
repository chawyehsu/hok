use anyhow::Result;
use crate::Scoop;
use regex::Regex;

struct SearchMatch {
  name: String,
  version: String,
  bin: Option<String>
}

impl Scoop {
  pub fn search(&self, query: &str) -> Result<()> {
    let re = Regex::new(query)?;
    let buckets = self.get_added_buckets()?;

    for bucket in buckets {
      let mut matches: Vec<SearchMatch> = Vec::new();

      if let Some(apps) = self.apps_in_bucket(bucket.as_str())? {
        for app in apps.iter() {
          let app_name = app.file_name();
          let app_name = app_name.to_str().unwrap().trim_end_matches(".json");
          if re.is_match(app_name) {
            let manifest = self.manifest_from_local(app.path())?;
            let version = manifest.get("version");

            // filter bad manifest doesn't contain `version`
            if version.is_none() {
              continue;
            }

            let name = app_name.to_string();
            let version = version.unwrap().as_str().unwrap().to_owned();
            let bin: Option<String> = None; // FIXME

            let sm = SearchMatch {
              name,
              version,
              bin
            };

            matches.push(sm);
          }
        }
      }

      if matches.len() > 0 {
        println!("'{}' bucket:", bucket);
        for sm in matches {
          if sm.bin.is_none() {
            println!("  {} ({})", sm.name, sm.version);
          } else {
            println!("  {} ({}) --> includes {:?}", sm.name, sm.version, sm.bin);
          }
        }
        println!("");
      }
    }

    Ok(())
  }
}
