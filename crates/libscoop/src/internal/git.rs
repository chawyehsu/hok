use git2::{CredentialType, FetchOptions, Repository};
use std::{path::Path, result::Result};

use crate::error::Fallible;

fn fetch_options(proxy: Option<&str>) -> FetchOptions<'static> {
    let mut fo = FetchOptions::new();
    let mut cb = git2::RemoteCallbacks::new();

    cb.credentials(
        move |url, username, cred| -> Result<git2::Cred, git2::Error> {
            let user = username.unwrap_or("git");
            let cfg = &(git2::Config::open_default()?);

            if cred.contains(CredentialType::USERNAME) {
                git2::Cred::username(user)
            } else if cred.contains(CredentialType::USER_PASS_PLAINTEXT) {
                git2::Cred::credential_helper(cfg, url, username)
            } else if cred.contains(CredentialType::DEFAULT) {
                git2::Cred::default()
            } else {
                Err(git2::Error::from_str("no authentication available"))
            }
        },
    );

    fo.remote_callbacks(cb);

    if let Some(proxy) = proxy {
        let mut proxy = proxy.to_owned();

        if !(proxy.starts_with("http") || proxy.starts_with("socks")) {
            proxy.insert_str(0, "http://");
        }

        let mut po = git2::ProxyOptions::new();
        po.url(proxy.as_str());
        fo.proxy_options(po);
    }

    fo
}

pub fn clone_repo<S, P>(remote_url: S, path: P, proxy: Option<S>) -> Fallible<()>
where
    S: AsRef<str>,
    P: AsRef<Path>,
{
    let proxy = proxy.as_ref().map(|s| s.as_ref());
    let mut repo_builder = git2::build::RepoBuilder::new();
    repo_builder.fetch_options(fetch_options(proxy));

    repo_builder
        .clone(remote_url.as_ref(), path.as_ref())
        .map_err(|e| e.into())
        .map(|_| {})
}

// NOTE: this will discard all local changes.
pub fn reset_head<P, S>(path: P, proxy: Option<S>) -> Fallible<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let proxy = proxy.as_ref().map(|s| s.as_ref());
    let repo = Repository::open(path.as_ref())?;
    let mut origin = repo.find_remote("origin")?;

    // fetch all refs
    origin.fetch(
        &["refs/heads/*:refs/heads/*"],
        Some(&mut fetch_options(proxy)),
        None,
    )?;

    let head = repo.head()?.target().unwrap();
    let obj = repo.find_object(head, None)?;

    repo.reset(&obj, git2::ResetType::Hard, None)?;

    Ok(())
}

pub fn remote_url_of<S>(repo_path: &Path, remote: S) -> Fallible<Option<String>>
where
    S: AsRef<str>,
{
    let repo = Repository::open(repo_path)?;
    let remote = repo.find_remote(remote.as_ref())?;
    Ok(remote.url().map(|s| s.to_owned()))
}
