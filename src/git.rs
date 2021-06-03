use crate::config::Config;
use git2::{Repository, ProxyOptions};
use std::{io::Write, path::Path};
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct GitTool {
  proxy: Option<String>
}

impl GitTool {
  pub fn new(config: &Config) -> GitTool {
    match config.get("proxy") {
      Some(proxy) => {
        let mut proxy = proxy.as_str().unwrap().to_string();

        if !proxy.starts_with("http") {
          proxy.insert_str(0, "http://");
        }

        GitTool { proxy: Some(proxy) }
      },
      None => GitTool { proxy: None }
    }
  }

  fn fetch_options(&self) -> git2::FetchOptions {
    let mut fo = git2::FetchOptions::new();
    let mut cb = git2::RemoteCallbacks::new();

    cb.credentials(move |url, username, cred|
      -> Result<git2::Cred, git2::Error> {

      // println!("{:?} {:?} {:?}", url, username, cred);
      let user = username.unwrap_or("git");
      let ref cfg = git2::Config::open_default()?;

      if cred.contains(git2::CredentialType::USERNAME) {
        git2::Cred::username(user)
      } else if cred.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
        git2::Cred::credential_helper(cfg, url, username)
      } else if cred.contains(git2::CredentialType::DEFAULT) {
        git2::Cred::default()
      } else {
        Err(git2::Error::from_str("no authentication available"))
      }
    });

    fo.remote_callbacks(cb);

    match &self.proxy {
      Some(proxy) => {
        let mut po = ProxyOptions::new();
        po.url(proxy.as_str());
        fo.proxy_options(po);
      },
      None => {}
    }

    fo
  }

  fn repo_builder(&self) -> git2::build::RepoBuilder {
    let mut repo_builder = git2::build::RepoBuilder::new();
    repo_builder.fetch_options(self.fetch_options());

    repo_builder
  }

  pub fn clone<S: AsRef<str>, P: AsRef<Path>>(&self, local_path: P, remote_url: S) -> Result<()> {
    print!("Checking repo... ");
    std::io::stdout().flush().unwrap();

    let mut rb = self.repo_builder();
    match rb.clone(
      remote_url.as_ref(), local_path.as_ref()
    ) {
      Ok(_repo) => {
        print!("ok\n");
        std::io::stdout().flush().unwrap();
        println!("The {} bucket was added successfully.", local_path.as_ref().to_str().unwrap());
        return Ok(());
      },
      Err(_e) => return Err(anyhow!("Failed to clone repo {} as bucket.", remote_url.as_ref())),
    }
  }

  // NOTE: this will discard all local changes.
  pub fn reset_head<P: AsRef<Path>>(&self, path: P) -> Result<()> {
    let repo = Repository::open(path.as_ref())?;

    let mut origin = repo.find_remote("origin")?;
    // fetch origin all refs
    origin.fetch(
      &["refs/heads/*:refs/heads/*"],
      Some(&mut self.fetch_options()),
      None,
    )?;

    let head = repo.head()?.target().unwrap();
    let obj = repo.find_object(head, None)?;

    repo.reset(&obj, git2::ResetType::Hard,None)?;

    Ok(())
  }
}
