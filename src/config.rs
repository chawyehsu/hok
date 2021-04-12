use dirs;
use serde_json::Value;

pub fn load_cfg() -> Value {
  let cfg_file = dirs::home_dir()
    .map(|p| p.join(".config\\scoop\\config.json"))
    .unwrap();

  // println!("{}", cfg_file.to_str().unwrap());

  let cfg_val: Value;
  if let Ok(s) = std::fs::read_to_string(cfg_file) {
    cfg_val = serde_json::from_str(s.as_str()).unwrap();
  } else {
    cfg_val = serde_json::from_str("{}").unwrap();
  }

  cfg_val
}
