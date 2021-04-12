use crate::Scoop;
use git2::build::RepoBuilder;
use std::io::Write;
use serde_json::Value;

impl Scoop {

  fn repo_builder(&self) -> RepoBuilder {
    let mut repo_builder = RepoBuilder::new();

    match self.config["proxy"].clone() {
      Value::String(mut proxy) => {
        let mut fo = git2::FetchOptions::new();
        let mut po = git2::ProxyOptions::new();

        if !proxy.starts_with("http") {
          proxy.insert_str(0, "http://");
        }

        po.url(proxy.as_str());
        fo.proxy_options(po);

        repo_builder.fetch_options(fo);
      },
      _ => {}
    }

    repo_builder
  }

  pub fn clone(&self, bucket_name: &str, bucket_url: &str) {
    print!("Checking repo... ");
    std::io::stdout().flush().unwrap();

    let mut rb = self.repo_builder();
    match rb.clone(
      bucket_url, &self.buckets_dir.join(bucket_name)
    ) {
      Ok(repo) => {
        print!("ok\n");
        std::io::stdout().flush().unwrap();
        println!("The {} bucket was added successfully.", bucket_name);
        repo
      },
      Err(e) => panic!("failed to clone: {}", e),
    };
  }
}
