use curl::easy::Easy;
use std::time::Duration;

pub fn get_content_length(url: &str, proxy: Option<&str>) -> Option<f64> {
    let mut easy = Easy::new();
    easy.get(true).unwrap();
    easy.url(url).unwrap();
    if let Some(proxy) = proxy {
        easy.proxy(proxy).unwrap();
    }
    easy.nobody(true).unwrap();
    easy.follow_location(true).unwrap();
    easy.connect_timeout(Duration::from_secs(30)).unwrap();
    println!("get_content_length of: {}", url);
    easy.perform().unwrap();
    easy.content_length_download().ok()
}
