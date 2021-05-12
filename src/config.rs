use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

use dirs;
use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::{Map, Value};
use crate::Scoop;

static CONCFG_PATH: Lazy<PathBuf> = Lazy::new(|| {
  dirs::home_dir().map(|p|
    p.join(".config\\scoop\\config.json")).unwrap()
});

pub fn load_cfg() -> Result<Value> {
  let file = File::open(CONCFG_PATH.as_path());

  match file {
    Ok(file) => {
      let reader = BufReader::new(file);

      match serde_json::from_reader(reader) {
        Ok(val) => {
          let cfg = match val {
            Value::Object(m) => {
              m.into_iter().map(|(k, v)| (k.to_ascii_lowercase(), v))
                .collect::<Map<String, Value>>()
            },
            _ => {
              Map::new()
            }
          };

          Ok(serde_json::to_value(cfg).unwrap())
        },
        Err(_e) => {
          println!("{}", format!("Failed to parse config file '{}'",
          CONCFG_PATH.to_str().unwrap()));
          Ok(serde_json::from_str("{}").unwrap())
        }
      }
    },
    Err(_e) => {
      Ok(serde_json::from_str("{}").unwrap())
    }
  }
}

impl Scoop {
  pub fn get_config(&mut self, key: &str) -> Option<&Value> {
    let k = key.to_ascii_lowercase();

    self.config.get(k)
  }

  pub fn set_config(&mut self, key: &str, value: &str) -> Result<()> {
    let k = key.to_ascii_lowercase();

    if value.eq("null") || value.eq("none") { // FIXME
      Map::remove(self.config.as_object_mut().unwrap(), &k);
    } else if value.parse::<u64>().is_ok() {
      self.config[k] = Value::from(value.parse::<u64>()?);
    } else if value.eq("true") || value.eq("false") {
      self.config[k] = Value::from(value.parse::<bool>()?);
    } else {
      self.config[k] = Value::from(value);
    }

    // Ensure config directory exists
    crate::fs::ensure_dir(CONCFG_PATH.parent().unwrap())?;

    // Read or create config file
    let file = OpenOptions::new()
      .write(true).create(true).open(CONCFG_PATH.as_path());

    match file {
      Ok(file) => {
        let buffer = BufWriter::new(file);
        serde_json::to_writer_pretty(buffer, &self.config)
        .expect("Failed to save configs.");
        return Ok(());
      },
      Err(_e) => return Err(
        anyhow::anyhow!("Failed to create config file.")
      )
    }
  }
}

