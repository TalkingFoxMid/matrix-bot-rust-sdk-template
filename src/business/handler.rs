use super::state::State;

/// The main business logic handler
///
/// This function implements the actor model pattern:
/// - Takes user input (message text) and current state
/// - Returns bot responses (vector of strings) and new state
/// - All business logic decisions happen here
///
/// # Arguments
/// * `input` - The text message from the user
/// * `state` - The current state of the bot
///
/// # Returns
/// A tuple of (responses, new_state) where:
/// - responses: Vec<String> - Messages to send back (can be empty, one, or many)
/// - new_state: State - The updated state after processing this input
pub async fn handle(input: String, state: State) -> (Vec<String>, State) {
    let mut responses = Vec::new();
    let mut new_state = state;

    // Command: !party
    if input.contains("!party") {
        responses.push("🎉🎊🥳 let's PARTY!! 🥳🎊🎉".to_string());
    }

    // Add more commands here as the bot grows
    // if input.starts_with("!help") { ... }
    // if input.starts_with("!stats") { ... }

    (responses, new_state)
}
