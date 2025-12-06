/// Core infrastructure module
///
/// This module contains all infrastructure concerns:
/// - Authentication and session management
/// - Verification handling
/// - Sync and auto-join functionality
///
/// Business logic should not be placed here.

pub mod auth;
pub mod verification;
pub mod sync;

// Re-export commonly used functions
pub use auth::{login, setup_cross_signing};
pub use verification::{on_verification_request, handle_verification};
pub use sync::on_stripped_state_member;
