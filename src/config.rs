use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use dirs;
use lazy_static::lazy_static;
use serde_json::{Map, Value};
use crate::Scoop;

lazy_static! {
  static ref CONCFG_PATH: PathBuf = dirs::home_dir()
  .map(|p| p.join(".config\\scoop\\config.json"))
  .unwrap();
}

pub fn load_cfg() -> Value {
  let mut cfg_val: Value = serde_json::from_str("{}").unwrap();
  if let Ok(s) = std::fs::read_to_string(CONCFG_PATH.as_path()) {
    match serde_json::from_str(s.as_str()) {
      Ok(t) => { cfg_val = t },
      Err(e) => {
        println!("Failed to parse '{}', remove the file or check its format.",
          CONCFG_PATH.as_os_str().to_str().unwrap().to_string());
        panic!("{}", e);
      }
    }
  }

  serde_json::to_value(
    cfg_val
      .as_object()
      .unwrap()
      .to_owned()
      .into_iter()
      .map(|(k, v)| (k.to_lowercase(), v))
      .collect::<Map<String, Value>>()
  ).unwrap()
}

impl Scoop {
  pub fn get_config(&mut self, key: &str) -> String {
    let lower_key = key.to_lowercase();
    let k = lower_key.as_str();

    if self.config[k].is_null() {
      self.set_config(k, "null");
      "null".to_string()
    } else {
      self.config[k].to_string()
    }
  }

  pub fn set_config(&mut self, key: &str, value: &str) {
    let lower_key = key.to_lowercase();
    let k = lower_key.as_str();

    if value.eq("null") || value.eq("none") { // FIXME
      if self.config.is_object() {
        Map::remove(self.config.as_object_mut().unwrap(), k);
      }
    } else if value.parse::<u64>().is_ok() {
      self.config[k] = Value::from(value.parse::<u64>().unwrap());
    } else if value.eq("true") || value.eq("false") {
      self.config[k] = Value::from(value.parse::<bool>().unwrap());
    } else {
      self.config[k] = Value::from(value);
    }

    let buffer = BufWriter::new(File::create(CONCFG_PATH.as_path()).unwrap());
    serde_json::to_writer_pretty(buffer, &self.config)
      .expect("failed to save configs.");
  }
}

