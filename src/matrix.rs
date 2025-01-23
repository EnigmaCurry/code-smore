#![allow(unused_imports)]
use crate::prelude::*;
use log::{error, info};
#[cfg(feature = "matrix")]
use matrix_sdk::ruma::events::room::message::{
    MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
};
#[cfg(feature = "matrix")]
use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId, RoomId};
#[cfg(feature = "matrix")]
use matrix_sdk::RoomState;
#[cfg(feature = "matrix")]
use matrix_sdk::{config::SyncSettings, room::edit::EditedContent, room::Room, Client};
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Debug, EnumString, EnumIter, Display)]
enum BotCommand {
    #[strum(serialize = "morse-enable")]
    MorseEnable,
    #[strum(serialize = "morse-disable")]
    MorseDisable,
    #[strum(serialize = "status")]
    Status,
    #[strum(serialize = "help")]
    Help,
}

#[cfg(feature = "matrix")]
#[allow(dead_code)]
fn get_command_help(command: &BotCommand) -> &'static str {
    match command {
        BotCommand::MorseEnable => "Enable Morse processing.",
        BotCommand::MorseDisable => "Disable Morse processing.",
        BotCommand::Status => "Display the current status of Morse processing.",
        BotCommand::Help => "Display this help message.",
    }
}

#[cfg(feature = "matrix")]
#[allow(dead_code)]
fn parse_command(message: &str) -> Option<(BotCommand, Vec<String>)> {
    if !message.starts_with('!') {
        return None;
    }

    let parts: Vec<&str> = message[1..].split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let command_str = parts[0];
    let args = parts[1..].iter().map(|s| s.to_string()).collect();

    match command_str.parse::<BotCommand>() {
        Ok(command) => Some((command, args)),
        Err(_) => None,
    }
}

#[cfg(feature = "matrix")]
#[allow(dead_code)]
async fn bridge_stdout(
    client: Client,
    room_id: Arc<OwnedRoomId>,
    exe_path: PathBuf,
    gpio_pin: Option<u8>,
) -> anyhow::Result<()> {
    let mut child: Child;
    if let Some(gpio_pin) = gpio_pin {
        child = Command::new(&exe_path)
            .arg("receive")
            .arg("--gpio")
            .arg(format!("{gpio_pin}"))
            .arg("--buffered")
            .stdout(Stdio::piped())
            .spawn()?;
    } else {
        error!("Only GPIO bridging supported right now.");
        std::process::exit(1);
    }

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut reader = BufReader::new(stdout);
    let mut buf = String::new();
    let mut interim_message_event_id: Option<OwnedEventId> = None;

    loop {
        buf.clear();
        let bytes_read = reader.read_line(&mut buf).await?;
        if bytes_read == 0 {
            break; // End of stream
        }

        info!("line: {buf:?}");

        if buf == "\n".to_string() && interim_message_event_id.is_none() {
            buf.clear();
            // Send the "Receiving Message ...." notification
            if let Some(room) = client.get_room(&room_id) {
                match room
                    .send(RoomMessageEventContent::text_plain(
                        "Receiving Message ....",
                    ))
                    .await
                {
                    Ok(response) => interim_message_event_id = Some(response.event_id),
                    Err(err) => error!("Failed to send interim message: {}", err),
                }
            } else {
                error!("Failed to find joined room with ID: {}", room_id);
            }
        } else if buf.ends_with('\n') {
            // Full line received, process normally
            info!("Received from code-smore: {}", buf);

            if let Some(room) = client.get_room(&room_id) {
                if let Some(event_id) = &interim_message_event_id {
                    // Create an edit event with the new content
                    let edited_content = EditedContent::RoomMessage(
                        RoomMessageEventContent::text_plain(&buf).into(),
                    );
                    match room.make_edit_event(event_id, edited_content).await {
                        Ok(edited_message_content) => {
                            if let Err(err) = room.send(edited_message_content).await {
                                error!("Failed to send edited message to room: {}", err);
                            }
                        }
                        Err(err) => error!("Failed to create edit event: {}", err),
                    }
                    interim_message_event_id = None;
                } else {
                    // If there's no interim message, send a new one
                    if let Err(err) = room.send(RoomMessageEventContent::text_plain(&buf)).await {
                        error!("Failed to send message to room: {}", err);
                    }
                }
            } else {
                error!("Failed to find joined room with ID: {}", room_id);
            }
        }
    }

    Ok(())
}
#[cfg(feature = "matrix")]
#[allow(dead_code)]
pub async fn main() -> anyhow::Result<()> {
    let homeserver_url = env::var("MATRIX_HOMESERVER").expect("MATRIX_HOMESERVER is not set");
    let username = env::var("MATRIX_USERNAME").expect("MATRIX_USERNAME is not set");
    let password = env::var("MATRIX_PASSWORD").expect("MATRIX_PASSWORD is not set");
    let room_id = env::var("MATRIX_ROOM_ID").expect("MATRIX_ROOM_ID is not set");
    let room_id = Arc::new(RoomId::parse(&room_id).expect("Invalid room id"));
    let exe_path = env::current_exe().expect("Failed to get the current executable path");

    let client = Client::builder()
        .homeserver_url(homeserver_url)
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(username, &password)
        .send()
        .await?;
    info!("Logged in as {}", client.user_id().unwrap());

    client.join_room_by_id(&room_id).await?;
    info!("Joined room {}", room_id);

    let process_lock = Arc::new(Mutex::new(()));
    let morse_enabled = Arc::new(AtomicBool::new(true));
    let gpio_pin = 17;

    tokio::spawn(bridge_stdout(
        client.clone(),
        room_id.clone(),
        exe_path.clone(),
        Some(gpio_pin),
    ));

    client.sync_once(SyncSettings::default()).await?;
    info!("Matrix client synced");

    let process_lock_clone = Arc::clone(&process_lock);
    let morse_enabled_clone = Arc::clone(&morse_enabled);
    let session_meta = client.session_meta().expect("Invalid matrix session");
    let user_id = session_meta.user_id.clone();

    client.add_event_handler(move |event: OriginalSyncRoomMessageEvent, room: Room| {
        let process_lock = Arc::clone(&process_lock_clone);
        let morse_enabled = Arc::clone(&morse_enabled_clone);

        async move {
            if room.state() != RoomState::Joined {
                return;
            } else {
                if event.sender == *user_id {
                    // Ignore our own messages
                    return;
                }
                if let MessageType::Text(text_content) = event.content.msgtype {
                    if let Some((command, _args)) = parse_command(&text_content.body) {
                        match command {
                            BotCommand::MorseEnable => {
                                morse_enabled.store(true, Ordering::SeqCst);
                                info!("Morse processing enabled.");
                                if let Err(err) = room
                                    .send(RoomMessageEventContent::text_plain(
                                        "Morse processing has been enabled.",
                                    ))
                                    .await
                                {
                                    error!("Failed to send response: {}", err);
                                }
                            }
                            BotCommand::MorseDisable => {
                                morse_enabled.store(false, Ordering::SeqCst);
                                info!("Morse processing disabled.");
                                if let Err(err) = room
                                    .send(RoomMessageEventContent::text_plain(
                                        "Morse processing has been disabled.",
                                    ))
                                    .await
                                {
                                    error!("Failed to send response: {}", err);
                                }
                            }
                            BotCommand::Status => {
                                let status = if morse_enabled.load(Ordering::SeqCst) {
                                    "enabled"
                                } else {
                                    "disabled"
                                };
                                let response = format!("Morse processing is currently {}.", status);
                                info!("{}", response);
                                if let Err(err) = room
                                    .send(RoomMessageEventContent::text_plain(&response))
                                    .await
                                {
                                    error!("Failed to send response: {}", err);
                                }
                            }
                            BotCommand::Help => {
                                let help_message = BotCommand::iter()
                                    .map(|cmd| format!("!{}: {}", cmd, get_command_help(&cmd)))
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                if let Err(err) = room
                                    .send(RoomMessageEventContent::text_plain(format!(
                                        "Available commands:\n{}",
                                        help_message
                                    )))
                                    .await
                                {
                                    error!("Failed to send help response: {}", err);
                                }
                            }
                        }
                        return;
                    }

                    if text_content.body.starts_with('!') {
                        error!("Unknown command: {}", text_content.body);
                        if let Err(err) = room
                            .send(RoomMessageEventContent::text_plain(
                                "Unknown command. Use `!help` to see available commands.",
                            ))
                            .await
                        {
                            error!("Failed to send response: {}", err);
                        }
                        return;
                    }

                    if !morse_enabled.load(Ordering::SeqCst) {
                        info!(
                            "Morse processing is disabled. Ignoring message: {:?}",
                            text_content
                        );
                        return;
                    }

                    info!("Bridging message: {:?}", text_content);

                    let _lock = process_lock.lock().await;

                    let mut child = Command::new(&exe_path)
                        .arg("send")
                        .arg("--gpio")
                        .arg("4")
                        .stdin(Stdio::piped())
                        .spawn()
                        .expect("Failed to spawn code-smore send command");
                    if let Some(mut stdin) = child.stdin.take() {
                        use tokio::io::AsyncWriteExt;
                        if let Err(err) = stdin.write_all(text_content.body.as_bytes()).await {
                            error!("Failed to write to code-smore stdin: {}", err);
                        }
                    }
                    let status = child.wait().await;
                    match status {
                        Ok(status) if status.success() => {
                            info!("Message sent to GPIO successfully.");
                        }
                        Ok(status) => {
                            error!("Command failed with status: {}", status);
                        }
                        Err(err) => {
                            error!("Error running command: {}", err);
                        }
                    }
                }
            }
        }
    });

    info!("Bot is listening...");
    client.sync(SyncSettings::default()).await?;
    Ok(())
}
