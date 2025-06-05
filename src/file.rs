use std::fs::File;
use std::io::BufReader;
use std::sync::mpsc::Sender;

use crate::{fft::compute_fft_magnitude, fft::hann_window, message::Message, morse::text_to_morse};
use hound;
use morse_codec::decoder::Decoder;
use regex::Regex;

const DEBOUNCE_MS: u32 = 30;

pub fn listen_to_file(
    file_name: &str,
    tone_freq: f32,
    _bandwidth: f32,
    threshold: f32,
    dot_duration: u32,
    output_morse: bool,
    tx: Option<Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Opening file: {}", file_name);
    let mut reader = hound::WavReader::new(BufReader::new(File::open(file_name)?))?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;
    println!("Sample rate: {}", sample_rate);

    let mut decoder = Decoder::<9999>::new()
        .with_reference_short_ms(dot_duration as u16)
        .build();

    let whitespace_regex = Regex::new(r"\\s+").unwrap();
    let mut last_signal_state = false;
    let mut message_log = Vec::new();

    let window_size = 1024;
    let target_bin = ((tone_freq / sample_rate) * window_size as f32).round() as usize;
    println!(
        "Target bin: {}/{} for tone {} Hz",
        target_bin, window_size, tone_freq
    );
    let fft_window = hann_window(window_size);
    let mut buffer = Vec::new();
    let mut sample_clock: usize = 0;
    let mut last_signal_change_sample: usize = 0;

    for sample in reader.samples::<i16>() {
        let s = sample? as f32 / i16::MAX as f32;
        buffer.push(s);
        sample_clock += 1;

        if buffer.len() >= window_size {
            let magnitude = compute_fft_magnitude(&buffer[..window_size], &fft_window, target_bin);
            let tone_detected = magnitude > threshold;

            let duration_samples = sample_clock - last_signal_change_sample;
            let duration = (duration_samples as f32 / sample_rate * 1000.0).round() as u32;

            if tone_detected && !last_signal_state && duration < DEBOUNCE_MS {
                buffer.drain(..window_size / 2);
                continue;
            }

            if tone_detected != last_signal_state {
                decoder.signal_event(duration as u16, last_signal_state);
                let mut msg = decoder.message.as_str().to_string();
                msg = whitespace_regex.replace_all(&msg, " ").to_string();

                if let Some(tx) = &tx {
                    if !msg.trim().is_empty() {
                        let _ = tx.send(format!(":typing:{}", msg));
                    }
                }

                last_signal_state = tone_detected;
                last_signal_change_sample = sample_clock;
            }

            if duration > 20 * dot_duration {
                last_signal_change_sample = sample_clock;
                last_signal_state = false;
                decoder.signal_event_end(false);
                decoder.signal_event_end(true);
                let mut msg = decoder.message.as_str().to_string();
                msg = whitespace_regex.replace_all(&msg, " ").trim().to_string();

                if !msg.is_empty() {
                    let m = Message {
                        timestamp: format!("{:.3} sec", sample_clock as f32 / sample_rate),
                        content: if output_morse {
                            text_to_morse(&msg)
                        } else {
                            msg.clone()
                        },
                    };

                    if let Some(ref tx) = tx {
                        if !m.content.trim().is_empty() {
                            let _ = tx.send(m.content.clone());
                        }
                    }

                    println!("[{}] > {}", m.timestamp, m.content);
                    message_log.push(m.clone());
                    decoder.message.clear();
                }
            }

            buffer.drain(..window_size / 2);
        }
    }

    decoder.signal_event_end(last_signal_state);
    decoder.signal_event_end(!last_signal_state);
    let final_msg = decoder.message.as_str().to_string();
    if !final_msg.trim().is_empty() {
        println!("Final message: {}", final_msg);
    }

    Ok(())
}
