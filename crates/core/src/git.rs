use crate::Config;
use crate::ScoopResult;
use std::{path::Path, result};

pub struct Git {
    proxy: Option<String>,
}

impl Git {
    pub fn new(config: &Config) -> Git {
        let proxy = config.proxy.clone();
        Git { proxy }
    }

    fn fetch_options(&self) -> git2::FetchOptions {
        let mut fo = git2::FetchOptions::new();
        let mut cb = git2::RemoteCallbacks::new();

        cb.credentials(
            move |url, username, cred| -> result::Result<git2::Cred, git2::Error> {
                log::trace!("{:?} {:?} {:?}", url, username, cred);
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
            },
        );

        fo.remote_callbacks(cb);

        if self.proxy.is_some() {
            let mut proxy = self.proxy.clone().unwrap();
            // prepend protocol if it does not exist
            if !(proxy.starts_with("http") || proxy.starts_with("socks")) {
                proxy.insert_str(0, "http://");
            }

            let mut po = git2::ProxyOptions::new();
            po.url(proxy.as_str());
            fo.proxy_options(po);
        }

        fo
    }

    fn repo_builder(&self) -> git2::build::RepoBuilder {
        let mut repo_builder = git2::build::RepoBuilder::new();
        repo_builder.fetch_options(self.fetch_options());

        repo_builder
    }

    pub fn clone<S, P>(&self, local_path: P, remote_url: S) -> ScoopResult<()>
    where
        S: AsRef<str>,
        P: AsRef<Path>,
    {
        self.repo_builder()
            .clone(remote_url.as_ref(), local_path.as_ref())
            .map_err(anyhow::Error::from)
            .map(|_| ())
    }

    // NOTE: this will discard all local changes.
    pub fn reset_head<P: AsRef<Path>>(&self, path: P) -> ScoopResult<()> {
        let repo = git2::Repository::open(path.as_ref())?;

        let mut origin = repo.find_remote("origin")?;
        // fetch origin all refs
        origin.fetch(
            &["refs/heads/*:refs/heads/*"],
            Some(&mut self.fetch_options()),
            None,
        )?;

        let head = repo.head()?.target().unwrap();
        let obj = repo.find_object(head, None)?;

        repo.reset(&obj, git2::ResetType::Hard, None)?;

        Ok(())
    }
}
