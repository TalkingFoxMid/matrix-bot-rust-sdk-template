///
///  This is an example showcasing how to build a very simple bot using the
/// matrix-sdk. To try it, you need a rust build setup, then you can run:
/// `cargo run -p example-getting-started -- <homeserver_url> <user> <password>`
///
/// Use a second client to open a DM to your bot or invite them into some room.
/// You should see it automatically join. Then post `!party` to see the client
/// in action.
///
// The imports we need
use std::{env, process::exit, sync::Arc};
use tokio::sync::Mutex;

use matrix_sdk::{Client, Room, RoomState, config::SyncSettings, ruma::events::room::{
    member::StrippedRoomMemberEvent,
    message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
}};
use matrix_sdk::ruma::events::key::verification::request::ToDeviceKeyVerificationRequestEvent;

mod core;
mod business;

use business::{State, INITIAL_STATE};

/// This is the starting point of the app. `main` is called by rust binaries to
/// run the program in this case, we use tokio (a reactor) to allow us to use
/// an `async` function run.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // set up some simple stderr logging. You can configure it by changing the env
    // var `RUST_LOG`
    tracing_subscriber::fmt::init();

    // parse the command line for homeserver, username and password
    let (Some(homeserver_url), Some(username), Some(password)) =
        (env::args().nth(1), env::args().nth(2), env::args().nth(3))
    else {
        eprintln!("Usage: {} <homeserver_url> <username> <password>", env::args().next().unwrap());
        // exit if missing
        exit(1)
    };

    // our actual runner
    login_and_sync(homeserver_url, &username, &password).await?;
    Ok(())
}

// The core sync loop we have running.
async fn login_and_sync(
    homeserver_url: String,
    username: &str,
    password: &str,
) -> anyhow::Result<()> {

    // First, we set up the client.
    let client = Client::builder()
        .homeserver_url(&homeserver_url)
        .sqlite_store("store.db", None)
        .build()
        .await?;

    // Login or restore session
    core::login(&client, username, password).await?;

    // Setup cross-signing for encryption
    core::setup_cross_signing(&client, username, password).await?;

    // Add verification request handler for to-device events
    client.add_event_handler(core::on_verification_request);

    println!("════════════════════════════════════════════════");
    println!("  Verification handlers registered!");
    println!("  Ready to accept verification requests!");
    println!("════════════════════════════════════════════════");

    // Initialize bot state
    let state = Arc::new(Mutex::new(INITIAL_STATE));

    // Auto-join on invite
    client.add_event_handler(core::on_stripped_state_member);

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store
    let sync_token = client.sync_once(SyncSettings::default()).await.unwrap().next_batch;

    // now that we've synced, let's attach a handler for incoming room messages
    let state_clone = state.clone();
    client.add_event_handler(move |event: OriginalSyncRoomMessageEvent, room: Room, client: Client| {
        let state = state_clone.clone();
        async move {
            on_room_message(event, room, client, state).await;
        }
    });

    // since we called `sync_once` before we entered our sync loop we must pass
    // that sync token to `sync`
    let settings = SyncSettings::default().token(sync_token);
    // this keeps state from the server streaming in to the bot via the
    // EventHandler trait
    client.sync(settings).await?; // this essentially loops until we kill the bot

    Ok(())
}

// This fn is called whenever we see a new room message event.
// It bridges infrastructure (Matrix events) with business logic (handler).
async fn on_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    client: Client,
    state: Arc<Mutex<State>>,
) {
    // Only process messages from joined rooms
    if room.state() != RoomState::Joined {
        return;
    }

    // Check if this is a verification request message
    let sender = &event.sender;
    let event_id = event.event_id.to_string();

    // Try to get a verification request using the event ID as flow_id
    if let Some(request) = client.encryption().get_verification_request(sender, &event_id).await {
        println!("╔══════════════════════════════════════════════╗");
        println!("║  IN-ROOM VERIFICATION REQUEST FOUND!         ║");
        println!("╚══════════════════════════════════════════════╝");
        println!("From: {}", sender);
        println!("Event ID (flow ID): {}", event_id);

        core::handle_verification(request, sender.to_string(), "in-room".to_string()).await;
        return;
    }

    // Extract text message
    let MessageType::Text(text_content) = event.content.msgtype else { return };

    // Get current state, call business handler, update state
    let (responses, new_state) = {
        let current_state = state.lock().await.clone();
        business::handle(text_content.body.clone(), current_state).await
    };

    // Update state
    {
        let mut state_guard = state.lock().await;
        *state_guard = new_state;
    }

    // Send all responses
    for response_text in responses {
        let content = RoomMessageEventContent::text_plain(&response_text);
        println!("sending: {}", response_text);

        if let Err(e) = room.send(content).await {
            eprintln!("Failed to send message: {e:?}");
        } else {
            println!("message sent");
        }
    }
}
