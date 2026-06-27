pub mod backends;
pub mod browsers;
pub mod config;
pub mod crypto;
pub mod history;
pub mod search;
pub mod security;
pub mod server;

#[cfg(feature = "desktop")]
mod desktop;

#[cfg(feature = "desktop")]
pub use desktop::run;