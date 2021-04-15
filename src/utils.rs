pub fn filesize(length: u64, with_unit: bool) -> String {
  let gb: f64 = 2.0_f64.powf(30_f64);
  let mb: f64 = 2.0_f64.powf(20_f64);
  let kb: f64 = 2.0_f64.powf(10_f64);

  let flength = length as f64;

  if flength > gb {
    let j = (flength / gb).round();

    if with_unit {
      format!("{}GB", j)
    } else {
      j.to_string()
    }
  } else if flength > mb {
    let j = (flength / mb).round();

    if with_unit {
      format!("{}MB", j)
    } else {
      j.to_string()
    }
  } else if flength > kb {
    let j = (flength / kb).round();

    if with_unit {
      format!("{}KB", j)
    } else {
      j.to_string()
    }
  } else {
    if with_unit {
      format!("{}B", flength)
    } else {
      flength.to_string()
    }
  }
}
