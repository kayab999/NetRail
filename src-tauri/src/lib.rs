pub mod backends;
pub mod browsers;
pub mod config;
pub mod crypto;
pub mod docs;
pub mod error;
pub mod history;
pub mod http_client;
pub mod search;
pub mod security;
pub mod server;

pub use error::{NetRailError, NetRailResult};

#[cfg(feature = "desktop")]
mod desktop;

#[cfg(feature = "desktop")]
pub use desktop::run;