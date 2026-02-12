//! Iron Core - Domain logic for Iron configuration management
//!
//! This crate contains the core business logic for Iron, including:
//! - Host management (hardware catalog, system config)
//! - Bundle management (desktop environments)
//! - Profile management (dotfile collections)
//! - Module management (individual components)
//! - State tracking and persistence

pub mod host;
pub mod bundle;
pub mod profile;
pub mod module;
pub mod state;
pub mod validation;

// Re-exports for convenience
pub use host::Host;
pub use bundle::Bundle;
pub use profile::Profile;
pub use module::Module;
