use crate::config::Config;
use reqwest::{
    blocking::{Client as ReqwestBlockingClient, RequestBuilder},
    IntoUrl, Method, Proxy, Result,
};

static SCOOP_USER_AGENT: &str = "Scoop/0.1.0 (Rust)";

#[derive(Debug)]
pub struct Client {
    inner: ReqwestBlockingClient,
}

impl Client {
    pub fn new(config: &Config) -> Result<Self> {
        let proxy = config.proxy.clone();
        let mut blocking_client = ReqwestBlockingClient::builder().user_agent(SCOOP_USER_AGENT);
        // Add proxy
        if proxy.is_some() {
            blocking_client = blocking_client.proxy(Proxy::all(proxy.unwrap().as_str())?)
        }

        Ok(Client {
            inner: blocking_client.build()?,
        })
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.request(Method::GET, url)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.request(Method::POST, url)
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.inner.request(Method::HEAD, url)
    }
}
