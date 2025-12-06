use anyhow::Result;
use matrix_sdk::Client;
use matrix_sdk::encryption::verification::{SasState, SasVerification, VerificationRequest, VerificationRequestState};
use matrix_sdk::ruma::events::key::verification::request::ToDeviceKeyVerificationRequestEvent;
use tokio::io::{AsyncBufReadExt, BufReader};
use futures_util::StreamExt;

/// Handle to-device verification requests
pub async fn on_verification_request(ev: ToDeviceKeyVerificationRequestEvent, client: Client) {
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

/// Common handler for both types of verification requests
pub async fn handle_verification(
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

/// Handle SAS (emoji) verification
async fn handle_sas_verification(sas: &SasVerification) -> Result<()> {
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
