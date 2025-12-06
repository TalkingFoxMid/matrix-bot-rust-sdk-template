# Matrix Bot Example

A production-ready Matrix bot built with Rust and the [matrix-sdk](https://github.com/matrix-org/matrix-rust-sdk), featuring clean architecture, encryption support, and an extensible command system.

## Features

### Core Infrastructure
- **Auto-join on invite** - Automatically joins rooms when invited
- **Session persistence** - Saves and restores login sessions to avoid re-authentication
- **SQLite state store** - Persistent storage for Matrix room state and encryption keys
- **Cross-signing setup** - Automatic encryption identity setup for E2E encrypted rooms
- **Emoji verification** - Interactive SAS (Short Authentication String) verification for device trust
- **Both to-device and in-room verification** - Supports verification requests via DMs or room messages

### Business Logic
- **Command system** - Extensible message command handling
- **Actor model pattern** - Pure functional state management
- **Clean separation** - Infrastructure and business logic are completely decoupled

### Current Commands
- `!party` - Celebrate with party emojis

## Architecture

The project follows a clean architecture with clear separation of concerns:

```
src/
├── main.rs              # Application entry point and sync loop
├── core/                # Infrastructure layer (Matrix protocol concerns)
│   ├── auth.rs         # Login, session management, cross-signing
│   ├── verification.rs # Device verification handling
│   └── sync.rs         # Auto-join and sync event handlers
└── business/            # Business logic layer (bot domain logic)
    ├── handler.rs      # Command processing and business logic
    └── state.rs        # Bot state definition
```

### Design Principles

**Core Module** (`src/core/`)
- Handles all Matrix protocol infrastructure
- Authentication, encryption, verification
- Sync and room membership
- **Never contains business logic**

**Business Module** (`src/business/`)
- Contains all bot-specific logic
- Implements the actor model pattern
- State is immutable and flows through the handler
- **Never touches Matrix SDK directly**

## Getting Started

### Prerequisites
- Rust toolchain (1.75+)
- A Matrix homeserver account

### Installation

```bash
# Clone the repository
git clone <your-repo-url>
cd matrix-examplum

# Build the project
cargo build --release
```

### Running the Bot

```bash
cargo run -- <homeserver_url> <username> <password>
```

Example:
```bash
cargo run -- https://matrix.org @mybot:matrix.org my_secure_password
```

On first run, the bot will:
1. Log in to the Matrix homeserver
2. Save the session to `session.json`
3. Setup encryption cross-signing
4. Start syncing and listening for messages

### Testing the Bot

1. Invite the bot to a room from another Matrix client
2. The bot will automatically join
3. Send a message: `!party`
4. The bot will respond with celebration emojis

## Adding Business Features

The bot is designed to make adding new features simple and maintainable. All business logic lives in the `business/` directory.

### Step 1: Define Your State (if needed)

Edit `src/business/state.rs`:

```rust
#[derive(Debug, Clone)]
pub struct State {
    pub party_count: u32,
    pub user_scores: HashMap<String, i32>,
    pub active_games: Vec<String>,
}

pub const INITIAL_STATE: State = State {
    party_count: 0,
    user_scores: HashMap::new(),
    active_games: Vec::new(),
};
```

### Step 2: Add Your Command Logic

Edit `src/business/handler.rs`:

```rust
pub async fn handle(input: String, mut state: State) -> (Vec<String>, State) {
    let mut responses = Vec::new();

    // Example: Counter command
    if input.starts_with("!count") {
        state.party_count += 1;
        responses.push(format!("Party count: {}", state.party_count));
    }

    // Example: Help command
    if input.starts_with("!help") {
        responses.push("Available commands:".to_string());
        responses.push("!party - Celebrate!".to_string());
        responses.push("!count - Increment counter".to_string());
        responses.push("!help - Show this message".to_string());
    }

    // Example: Dice roll (no state needed)
    if input.starts_with("!roll") {
        let roll = rand::random::<u8>() % 6 + 1;
        responses.push(format!("You rolled a {}!", roll));
    }

    // Example: Multi-response command
    if input.starts_with("!story") {
        responses.push("Once upon a time...".to_string());
        responses.push("There was a bot...".to_string());
        responses.push("The end!".to_string());
    }

    (responses, state)
}
```

### Command Patterns

#### Simple Response (Stateless)
```rust
if input.contains("!hello") {
    responses.push("Hello there!".to_string());
}
```

#### Command with State Mutation
```rust
if input.starts_with("!increment") {
    state.counter += 1;
    responses.push(format!("Counter: {}", state.counter));
}
```

#### Command with Arguments
```rust
if input.starts_with("!echo ") {
    let message = input.strip_prefix("!echo ").unwrap();
    responses.push(message.to_string());
}
```

#### Multiple Responses
```rust
if input.starts_with("!info") {
    responses.push("Bot Information".to_string());
    responses.push(format!("Uptime: {} messages", state.message_count));
    responses.push("Status: Online".to_string());
}
```

#### No Response (Silent Command)
```rust
if input.starts_with("!track") {
    // Just update state, don't send a message
    state.tracked_items.push(input.clone());
    // responses stays empty
}
```

### Step 3: Test Your Changes

```bash
cargo build
cargo run -- <homeserver_url> <username> <password>
```

Send your new command in a room with the bot and verify it works as expected.

## Actor Model Pattern

The bot uses the actor model for state management:

```
┌─────────────┐
│   Message   │
│   Received  │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────┐
│  handler(input, current_state)  │
│                                 │
│  • Pure function                │
│  • No side effects              │
│  • Returns new state            │
└──────────┬──────────────────────┘
           │
           ▼
    ┌──────────────────┐
    │ (responses, new) │
    │     state)       │
    └─────┬────────┬───┘
          │        │
          │        ▼
          │   ┌────────────┐
          │   │Update State│
          │   └────────────┘
          ▼
    ┌──────────┐
    │Send Msgs │
    └──────────┘
```

**Benefits:**
- Predictable state transitions
- Easy to test (pure functions)
- No shared mutable state bugs
- State changes are explicit

## Project Structure Details

### `main.rs`
- Application entry point
- Sets up the Matrix client with SQLite storage
- Registers event handlers
- Bridges Matrix events to business logic
- Manages the sync loop

### `core/auth.rs`
- Login with username/password
- Session persistence to `session.json`
- Session restoration on restart
- Cross-signing identity setup

### `core/verification.rs`
- Handles emoji verification requests
- Interactive terminal-based verification
- Supports both to-device and in-room verification

### `core/sync.rs`
- Auto-join functionality
- Retry logic with exponential backoff
- Handles room invitations

### `business/handler.rs`
- **Main entry point for all business logic**
- Takes input and state, returns responses and new state
- Add all your bot commands here

### `business/state.rs`
- Defines the bot's domain state
- Completely independent of Matrix infrastructure
- Add your state fields here

## Development Tips

1. **Keep business logic pure** - The `handler` function should be a pure function with no side effects

2. **State is immutable** - Always return a new state rather than mutating the old one

3. **Multiple responses are OK** - Return a vector of strings to send multiple messages

4. **Empty responses are OK** - Return an empty vector to update state without responding

5. **Use pattern matching** - Rust's pattern matching is great for command parsing

6. **Test in isolation** - Business logic is pure and easy to unit test

7. **Don't touch core/** - All infrastructure concerns are handled for you

## Examples of Features You Could Add

- **Game bot** - Trivia, word games, RPG mechanics
- **Utility bot** - Reminders, polls, voting
- **Integration bot** - GitHub notifications, RSS feeds, webhooks
- **Moderation bot** - Auto-mod, user warnings, logging
- **Info bot** - Knowledge base, FAQ, documentation lookup
- **Statistics bot** - Track messages, user activity, trends

## Encryption & Security

The bot automatically:
- Enables E2E encryption for supported rooms
- Sets up cross-signing identity
- Stores encryption keys securely in SQLite
- Supports device verification via emoji comparison

**Files created:**
- `session.json` - Login session (keep secure!)
- `store.db` - SQLite database with encryption keys and state

## Troubleshooting

**Bot doesn't respond to commands:**
- Check that the bot successfully joined the room
- Verify the room is in a "Joined" state
- Check console output for errors

**Verification fails:**
- Ensure both devices are online
- Try initiating verification from different devices
- Check that cross-signing is properly set up

**Session issues:**
- Delete `session.json` to force a fresh login
- Delete `store.db` if you need to reset encryption state
- Check homeserver connectivity

## License

Apache-2.0

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## Learn More

- [Matrix Rust SDK Documentation](https://docs.rs/matrix-sdk/)
- [Matrix Specification](https://spec.matrix.org/)
- [Matrix.org](https://matrix.org/)
