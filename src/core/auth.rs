use std::fs;
use std::path::Path;
use anyhow::Result;
use matrix_sdk::{Client, AuthSession};
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::{SessionMeta, SessionTokens};
use matrix_sdk::ruma::api::client::uiaa;
use matrix_sdk::ruma::api::client::uiaa::UserIdentifier;
use matrix_sdk::encryption::CrossSigningResetAuthType;
use serde_json::{json, Value};

const SESSION_FILE: &str = "session.json";

/// Login to Matrix server or restore existing session
///
/// This function handles:
/// - Restoring session from file if it exists
/// - Creating new login if no session exists
/// - Saving session to file
pub async fn login(
    client: &Client,
    username: &str,
    password: &str,
) -> Result<()> {
    let user_id = UserIdentifier::UserIdOrLocalpart(username.into());

    // Check if we have a saved session
    if Path::new(SESSION_FILE).exists() {
        println!("Restoring session from {SESSION_FILE}");
        let session_data = fs::read_to_string(SESSION_FILE)?;
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
            fs::write(SESSION_FILE, serde_json::to_string_pretty(&session_data)?)?;
            println!("Session saved to {SESSION_FILE}");
        }
    }

    Ok(())
}

/// Setup cross-signing for encryption
///
/// This ensures the client has proper encryption identity set up
pub async fn setup_cross_signing(
    client: &Client,
    username: &str,
    password: &str,
) -> Result<()> {
    let user_id = UserIdentifier::UserIdOrLocalpart(username.into());
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

    Ok(())
}
