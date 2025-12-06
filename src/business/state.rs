/// Business domain state
/// This state is modified only by the handler function
#[derive(Debug, Clone)]
pub struct State {
    // Add your business state fields here
    // For example: pub party_count: u32,
}

/// Initial state of the bot
pub const INITIAL_STATE: State = State {
    // Initialize your state fields here
};
