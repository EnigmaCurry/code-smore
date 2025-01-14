#![allow(unused_imports)]
use crate::morse::text_to_morse;
use crate::prelude::*;
use morse_codec::decoder::{Decoder, MorseDecoder};
#[cfg(feature = "gpio")]
use rppal;
use std::time::Duration;
use std::time::Instant;

#[allow(dead_code)]
fn get_decoder(dot_duration: u32) -> MorseDecoder<9999> {
    Decoder::<9999>::new()
        .with_reference_short_ms(dot_duration as u16)
        .build()
}

#[cfg(feature = "gpio")]
pub fn gpio_receive(
    dot_duration: u32,
    pin_number: u8,
    output_morse: bool,
    buffer_messages: bool,
) -> Result<(), std::io::Error> {
    let pin = rppal::gpio::Gpio::new()
        .expect("Failed to access GPIO")
        .get(pin_number)
        .expect("Failed to get GPIO pin")
        .into_input();

    let mut decoder = get_decoder(dot_duration);
    let mut last_signal_change = Instant::now();
    let mut last_signal_state = !pin.is_low(); // Normally high logic
    let mut message_pending = false; // Tracks if there's a pending message to finalize

    if !buffer_messages {
        clear_screen();
    }

    info!("Receiving morse code from GPIO pin {pin_number} - Press Ctrl-C to stop.");

    loop {
        let current_state = !pin.is_low(); // Invert the signal logic

        // Handle state changes
        if current_state != last_signal_state {
            let duration = last_signal_change.elapsed().as_millis();
            debug!(
                "State changed: {:?} -> {:?}, Duration: {} ms",
                last_signal_state, current_state, duration
            );
            decoder.signal_event(duration as u16, current_state);
            last_signal_change = Instant::now();
            last_signal_state = current_state;

            message_pending = true; // New signal indicates a valid message is being processed

            // Print the current message on the same line
            let message = decoder.message.as_str().trim().to_string();
            if !buffer_messages && !message.is_empty() {
                if output_morse {
                    print!("\r\x1b[K{}", text_to_morse(&message));
                } else {
                    print!("\r\x1b[K{message}");
                }
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        }

        // Check for inactivity
        let elapsed = last_signal_change.elapsed();
        if elapsed > Duration::from_millis(6 * 7 * dot_duration as u64) && message_pending {
            // Inactivity detected, finalize the pending message
            decoder.signal_event_end(false);
            let message = decoder.message.as_str().trim().to_string();
            if !message.is_empty() {
                debug!("Inactivity detected. Final message: {:?}", message);
                if buffer_messages {
                    println!("{message}");
                } else {
                    // Clear the current line before printing the final message
                    if output_morse {
                        print!("\r\x1b[K{}", text_to_morse(&message));
                    } else {
                        print!("\r\x1b[K{message}");
                    }
                    println!(); // Move to the next line after the final message
                }
            }
            message_pending = false; // Reset pending message flag
            decoder = get_decoder(dot_duration); // Reset decoder for a new message
        }

        // Prevent CPU overuse
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(not(feature = "gpio"))]
pub fn gpio_receive(
    _dot_duration: u32,
    _pin_number: u8,
    _output_morse: bool,
) -> Result<(), std::io::Error> {
    return Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "The GPIO feature is not enabled in this crate build",
    ));
}
