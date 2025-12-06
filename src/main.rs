///
///  This is an example showcasing how to build a very simple bot using the
/// matrix-sdk. To try it, you need a rust build setup, then you can run:
/// `cargo run -p example-getting-started -- <homeserver_url> <user> <password>`
///
/// Use a second client to open a DM to your bot or invite them into some room.
/// You should see it automatically join. Then post `!party` to see the client
/// in action.
///
/// Below the code has a lot of inline documentation to help you understand the
/// various parts and what they do
// The imports we need
use std::{env, process::exit};
use std::fs;
use std::path::Path;

use matrix_sdk::{Client, Room, RoomState, config::SyncSettings, ruma::events::room::{
    member::StrippedRoomMemberEvent,
    message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
}};
use matrix_sdk::encryption::CrossSigningResetAuthType;
use matrix_sdk::encryption::verification::{SasState, SasVerification, VerificationRequest, VerificationRequestState};
use matrix_sdk::ruma::api::client::uiaa;
use matrix_sdk::ruma::api::client::uiaa::UserIdentifier;
use matrix_sdk::ruma::events::key::verification::request::ToDeviceKeyVerificationRequestEvent;
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::{SessionMeta, SessionTokens, AuthSession};
use tokio::time::{Duration, sleep};
use tokio::io::{AsyncBufReadExt, BufReader};
use futures_util::StreamExt;
use serde_json::{json, Value};

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

    let user_id = UserIdentifier::UserIdOrLocalpart(username.into());
    let session_file = "session.json";

    // First, we set up the client.
    let client = Client::builder()
        .homeserver_url(&homeserver_url)
        .sqlite_store("store.db", None)
        .build()
        .await?;

    // Check if we have a saved session
    if Path::new(session_file).exists() {
        println!("Restoring session from {session_file}");
        let session_data = fs::read_to_string(session_file)?;
        let session: Value = serde_json::from_str(&session_data)?;

        // Restore the session
        let user_id_str = session["user_id"].as_str().unwrap();
        let device_id_str = session["device_id"].as_str().unwrap();
        let access_token = session["access_token"].as_str().unwrap().to_owned();
        let refresh_token = session["refresh_token"].as_str().map(|s| s.to_owned());

        client.restore_session(AuthSession::Matrix(MatrixSession {
            meta: SessionMeta {
                user_id: user_id_str.try_into()?,
                device_id: device_id_str.into(),
            },
            tokens: SessionTokens {
                access_token,
                refresh_token,
            },
        })).await?;

        println!("Session restored for {username}");
    } else {
        println!("No session file found, logging in...");

        // Then let's log that client in
        client
            .matrix_auth()
            .login_username(username, password)
            .initial_device_display_name("getting started bot")
            .await?;

        // It worked!
        println!("logged in as {username}");

        // Save the session
        if let Some(AuthSession::Matrix(session)) = client.session() {
            let session_data = json!({
                "user_id": session.meta.user_id.as_str(),
                "device_id": session.meta.device_id.as_str(),
                "access_token": session.tokens.access_token,
                "refresh_token": session.tokens.refresh_token,
            });
            fs::write(session_file, serde_json::to_string_pretty(&session_data)?)?;
            println!("Session saved to {session_file}");
        }
    }

    let encryption = client.encryption();

    let cross_signing_status = encryption.cross_signing_status().await.unwrap();

    if cross_signing_status.is_complete() {
        println!("Cross-signing is complete");
    } else {
        if let Some(handle) = encryption.recovery().reset_identity().await? {
            match handle.auth_type() {
                CrossSigningResetAuthType::Uiaa(uiaa) => {
                    let password = password.to_owned();
                    let mut password = uiaa::Password::new(user_id, password);
                    password.session = uiaa.session.clone();

                    handle.reset(Some(uiaa::AuthData::Password(password))).await?;
                }
                CrossSigningResetAuthType::OAuth(o) => {
                    println!(
                        "To reset your end-to-end encryption cross-signing identity,
                you first need to approve it at {}",
                        o.approval_url
                    );
                    handle.reset(None).await?;
                }
            }
        }
    }

    // Add verification request handler for to-device events
    client.add_event_handler(on_verification_request);

    println!("════════════════════════════════════════════════");
    println!("  Verification handlers registered!");
    println!("  Ready to accept verification requests!");
    println!("════════════════════════════════════════════════");

    // Now, we want our client to react to invites. Invites sent us stripped member
    // state events so we want to react to them. We add the event handler before
    // the sync, so this happens also for older messages. All rooms we've
    // already entered won't have stripped states anymore and thus won't fire
    client.add_event_handler(on_stripped_state_member);

    // An initial sync to set up state and so our bot doesn't respond to old
    // messages. If the `StateStore` finds saved state in the location given the
    // initial sync will be skipped in favor of loading state from the store
    let sync_token = client.sync_once(SyncSettings::default()).await.unwrap().next_batch;

    // now that we've synced, let's attach a handler for incoming room messages, so
    // we can react on it
    client.add_event_handler(on_room_message);

    // since we called `sync_once` before we entered our sync loop we must pass
    // that sync token to `sync`
    let settings = SyncSettings::default().token(sync_token);
    // this keeps state from the server streaming in to the bot via the
    // EventHandler trait
    client.sync(settings).await?; // this essentially loops until we kill the bot

    Ok(())
}

// Whenever we see a new stripped room member event, we've asked our client to
// call this function. So what exactly are we doing then?
async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
) {
    if room_member.state_key != client.user_id().unwrap() {
        // the invite we've seen isn't for us, but for someone else. ignore
        return;
    }

    // The event handlers are called before the next sync begins, but
    // methods that change the state of a room (joining, leaving a room)
    // wait for the sync to return the new room state so we need to spawn
    // a new task for them.
    tokio::spawn(async move {
        println!("Autojoining room {}", room.room_id());
        let mut delay = 2;

        while let Err(err) = room.join().await {
            // retry autojoin due to synapse sending invites, before the
            // invited user can join for more information see
            // https://github.com/matrix-org/synapse/issues/4345
            eprintln!("Failed to join room {} ({err:?}), retrying in {delay}s", room.room_id());

            sleep(Duration::from_secs(delay)).await;
            delay *= 2;

            if delay > 3600 {
                eprintln!("Can't join room {} ({err:?})", room.room_id());
                break;
            }
        }
        println!("Successfully joined room {}", room.room_id());
    });
}

// This fn is called whenever we see a new room message event. You notice that
// the difference between this and the other function that we've given to the
// handler lies only in their input parameters. However, that is enough for the
// rust-sdk to figure out which one to call and only do so, when the parameters
// are available.
async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room, client: Client) {
    // First, we need to unpack the message: We only want messages from rooms we are
    // still in and that are regular text messages - ignoring everything else.
    if room.state() != RoomState::Joined {
        return;
    }

    // Check if this is a verification request message
    // The SDK handles this internally, but we need to poll for the VerificationRequest object
    let sender = &event.sender;
    let event_id = event.event_id.to_string();

    // Try to get a verification request using the event ID as flow_id
    if let Some(request) = client.encryption().get_verification_request(sender, &event_id).await {
        println!("╔══════════════════════════════════════════════╗");
        println!("║  IN-ROOM VERIFICATION REQUEST FOUND!         ║");
        println!("╚══════════════════════════════════════════════╝");
        println!("From: {}", sender);
        println!("Event ID (flow ID): {}", event_id);

        handle_verification(request, sender.to_string(), "in-room".to_string()).await;
        return;
    }

    let MessageType::Text(text_content) = event.content.msgtype else { return };

    // here comes the actual "logic": when the bot see's a `!party` in the message,
    // it responds
    if text_content.body.contains("!party") {
        let content = RoomMessageEventContent::text_plain("🎉🎊🥳 let's PARTY!! 🥳🎊🎉");

        println!("sending");

        // send our message to the room we found the "!party" command in
        room.send(content).await.unwrap();

        println!("message sent");
    }
}

// Handle to-device verification requests
async fn on_verification_request(ev: ToDeviceKeyVerificationRequestEvent, client: Client) {
    println!("╔══════════════════════════════════════════════╗");
    println!("║  VERIFICATION REQUEST RECEIVED!              ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("From: {} (device: {})", ev.sender, ev.content.from_device);
    println!("Transaction ID: {}", ev.content.transaction_id);

    let request = client
        .encryption()
        .get_verification_request(&ev.sender, &ev.content.transaction_id)
        .await;

    if let Some(request) = request {
        handle_verification(request, ev.sender.to_string(), ev.content.from_device.to_string()).await;
    } else {
        eprintln!("Failed to get verification request object!");
    }
}

// Common handler for both types of verification requests
async fn handle_verification(
    request: VerificationRequest,
    sender: String,
    device_id: String,
) {
    tokio::spawn(async move {
        println!(
            "Received verification request from {} ({})",
            &sender,
            &device_id
        );

        request
            .accept()
            .await
            .expect("Can't accept verification request");

        let mut stream = request.changes();

        while let Some(state) = stream.next().await {
            match state {
                VerificationRequestState::Transitioned { verification } => {
                    if let Some(sas) = verification.sas() {
                        if let Err(e) = handle_sas_verification(&sas).await {
                            eprintln!("Error during SAS verification: {e:?}");
                        }
                    } else {
                        println!("Non-SAS verification methods are not supported in this bot");
                    }
                    break;
                }
                _ => (),
            }
        }
    });
}

// Handle SAS (emoji) verification
async fn handle_sas_verification(sas: &SasVerification) -> anyhow::Result<()> {
    println!("Starting emoji verification...");

    sas.accept().await?;

    let mut stream = sas.changes();

    while let Some(state) = stream.next().await {
        match state {
            SasState::KeysExchanged { emojis, decimals: _ } => {
                let emojis = emojis.expect("We only support emoji verification");

                println!("\n┌─────────────────────────────────────┐");
                println!("│  VERIFY THE FOLLOWING EMOJI MATCH:  │");
                println!("└─────────────────────────────────────┘");

                for (i, emoji) in emojis.emojis.iter().enumerate() {
                    print!("{} {}  ", emoji.symbol, emoji.description);
                    if (i + 1) % 4 == 0 {
                        println!();
                    }
                }
                println!();
                println!("\nDo the emoji match? (yes/no): ");

                // Read from stdin
                let stdin = tokio::io::stdin();
                let mut reader = BufReader::new(stdin);
                let mut input = String::new();

                reader.read_line(&mut input).await?;
                let input = input.trim().to_lowercase();

                if input == "yes" || input == "y" {
                    println!("Confirming verification...");
                    sas.confirm().await?;
                    println!("✓ Verification confirmed!");
                } else {
                    println!("Cancelling verification...");
                    sas.cancel().await?;
                    println!("✗ Verification cancelled");
                }
                break;
            }
            SasState::Done { .. } => {
                println!("✓ Verification completed successfully!");
                break;
            }
            SasState::Cancelled(cancel_info) => {
                println!("✗ Verification was cancelled: {:?}", cancel_info.reason());
                break;
            }
            _ => (),
        }
    }

    Ok(())
}
