/// Business logic module
///
/// This module contains all business domain logic for the bot.
/// It follows the actor model pattern where the handler function
/// is the single entry point for processing user commands.

pub mod state;
pub mod handler;

// Re-export main types for convenience
pub use state::{State, INITIAL_STATE};
pub use handler::handle;
