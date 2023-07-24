use flume::Sender;
use lazycell::LazyCell;
use std::cell::{Ref, RefCell, RefMut};
use std::path::Path;

use crate::{
    config::{default_config_path, Config, ConfigBuilder},
    error::{Error, Fallible},
    event::{Event, EventBus},
};

/// A handle representing a Scoop session.
#[derive(Debug)]
pub struct Session {
    /// [`Config`][1] for the session
    ///
    /// [1]: crate::config::Config
    config: RefCell<Config>,

    /// Full duplex channel for event transmission back and forth
    event_bus: LazyCell<EventBus>,

    /// User agent for the session
    pub(crate) user_agent: LazyCell<String>,
}

impl Session {
    /// Create a new session.
    ///
    /// The default config path will be used to locate the config file for the
    /// session.
    ///
    /// # Returns
    ///
    /// A new session.
    ///
    /// # Errors
    ///
    /// This method will return an error if the config file is not found or
    /// cannot be parsed.
    pub fn new() -> Fallible<Session> {
        let config_path = default_config_path();
        Self::new_with(config_path)
    }

    /// Create a new session with the given config path.
    ///
    /// # Returns
    ///
    /// A new session.
    ///
    /// # Errors
    ///
    /// This method will return an error if the config file is not found or
    /// cannot be parsed.
    pub fn new_with<P>(config_path: P) -> Fallible<Session>
    where
        P: AsRef<Path>,
    {
        let config = RefCell::new(ConfigBuilder::new(config_path).build()?);

        Ok(Session {
            config,
            event_bus: LazyCell::new(),
            user_agent: LazyCell::new(),
        })
    }

    /// Get an immutable reference to the config held by the session.
    ///
    /// This method is primarily used for doing a fine-grained read to the
    /// config aside from reading it as a whole via [`config_list`][1]. Caller
    /// of this method may not be able to perform some [`operations`][2], which
    /// will internally alter the config, before the reference is dropped.
    ///
    /// [1]: crate::operation::config_list
    /// [2]: crate::operation
    pub fn config(&self) -> Ref<Config> {
        self.config.borrow()
    }

    /// Get a mutable reference to the config held by the session.
    ///
    /// This method is only directly accessible from within the crate itself.
    /// It maybe indirectly used by other public available APIs to (indirectly)
    /// mutate the config. See [`Session::config`] for more details.
    pub(crate) fn config_mut(&self) -> Fallible<RefMut<Config>> {
        self.config.try_borrow_mut().map_err(|_| Error::ConfigInUse)
    }

    /// Get the event bus for the session.
    ///
    /// The event bus is used for transmitting [`events`][1] between the session
    /// backend and the caller frontend.
    ///
    /// # Returns
    ///
    /// The event bus for the session.
    ///
    /// [1]: crate::Event
    pub fn event_bus(&self) -> &EventBus {
        self.event_bus.borrow_with(EventBus::new)
    }

    /// Get an outbound sender to emit events.
    pub(crate) fn emitter(&self) -> Option<Sender<Event>> {
        self.event_bus.borrow().map(|bus| bus.inner_sender())
    }

    /// Set the user agent for the session.
    ///
    /// User agent is used when performing network related operations such as
    /// downloading packages. User agent for a session can only be set once.
    /// If not set, the default user agent will be used. The default user agent
    /// is `Scoop/1.0 (+http://scoop.sh/)`.
    ///
    /// # Errors
    ///
    /// This method will return an error if the user agent has already been set.
    pub fn set_user_agent(&self, user_agent: &str) -> Fallible<()> {
        self.user_agent
            .fill(user_agent.to_owned())
            .map_err(|_| Error::UserAgentAlreadySet)
    }
}
