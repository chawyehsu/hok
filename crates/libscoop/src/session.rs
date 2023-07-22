use lazycell::LazyCell;
use std::cell::{Ref, RefCell};
use std::path::Path;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::{
    config::{default_config_path, Config, ConfigBuilder},
    error::{Error, Fallible},
    event::Event,
};

/// A handle representing a Scoop session.
#[derive(Debug)]
pub struct Session {
    pub(crate) config: RefCell<Config>,
    pub(crate) emitter: UnboundedSender<Event>,
    pub(crate) user_agent: LazyCell<String>,
}

impl Session {
    pub fn init() -> Fallible<(Session, UnboundedReceiver<Event>)> {
        let config_path = default_config_path();
        Self::init_with(config_path)
    }

    pub fn init_with<P>(config_path: P) -> Fallible<(Session, UnboundedReceiver<Event>)>
    where
        P: AsRef<Path>,
    {
        let config = RefCell::new(ConfigBuilder::new(config_path).build()?);
        let (emitter, rx) = mpsc::unbounded_channel::<Event>();
        let s = Session {
            config,
            emitter,
            user_agent: LazyCell::new(),
        };
        Ok((s, rx))
    }

    pub fn get_config(&self) -> Ref<Config> {
        self.config.borrow()
    }

    pub fn set_user_agent(&self, user_agent: &str) -> Fallible<()> {
        self.user_agent
            .fill(user_agent.to_owned())
            .map_err(|_| Error::Custom("can only set useragent once".to_owned()))
    }
}
