use crate::{filter::BandpassFilter, message::Message, morse::text_to_morse, term::log_message};
use alsa::pcm::{Access, Format, HwParams, State};
use alsa::{Direction, PCM};
use chrono::Local;
use morse_codec::decoder::Decoder;
use regex::Regex;
use std::thread;
use std::time::Instant;

pub fn listen_with_alsa(
    device_name: &str,
    tone_freq: f32,
    bandwidth: f32,
    threshold: f32,
    dot_duration: u32,
    output_morse: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let pcm = PCM::new(device_name, Direction::Capture, false)?;

    // Set hardware parameters
    {
        let hwp = HwParams::any(&pcm)?;
        hwp.set_channels(1)?;
        hwp.set_rate(44100, alsa::ValueOr::Nearest)?;
        hwp.set_format(Format::s16())?;
        hwp.set_access(Access::RWInterleaved)?;
        pcm.hw_params(&hwp)?;
    }

    let io = pcm.io_i16()?;
    let frames = 1024;
    let mut buffer = vec![0i16; frames as usize];

    let mut decoder = Decoder::<9999>::new()
        .with_reference_short_ms(dot_duration as u16)
        .build();

    let mut filter = BandpassFilter::new(5, tone_freq as f64, bandwidth as f64, 44100.0)
        .expect("failed to create bandpass filter");

    let mut last_signal_state = false;
    let mut last_signal_change = Instant::now();
    let mut message_log = Vec::new();
    let whitespace_regex = Regex::new(r"\s+").unwrap();

    crate::term::clear_screen();

    loop {
        match io.readi(&mut buffer) {
            Ok(_) => {
                // Convert f32 -> f64
                let input: Vec<f64> = buffer.iter().map(|&s| s as f64 / i16::MAX as f64).collect();
                ///TODO: Filter this:
                //let filtered = filter.apply(&input);
                let filtered = input;

                // Compute tone energy
                let mut sum = 0.0_f32;
                let mut max: f32 = 0.0;
                let mut count = 0;

                for &s in &filtered {
                    let abs = s.abs() as f32;
                    sum += abs;
                    max = max.max(abs);
                    count += 1;
                }

                let average = if count > 0 { sum / count as f32 } else { 0.0 } * 30.0;
                let tone_detected = average > threshold;

                let now = Instant::now();
                let duration = now.duration_since(last_signal_change).as_millis() as u32;

                if tone_detected != last_signal_state {
                    decoder.signal_event(duration as u16, last_signal_state);
                    let mut msg = decoder.message.as_str().to_string();
                    msg = whitespace_regex.replace_all(&msg, " ").to_string();

                    if !msg.is_empty() {
                        crate::term::clear_screen();
                        for m in &message_log {
                            log_message(m);
                        }
                        if output_morse {
                            println!("{}", text_to_morse(&msg));
                        } else {
                            println!("{msg}");
                        }
                    }

                    last_signal_state = tone_detected;
                    last_signal_change = now;
                }

                // Message complete timeout
                if duration > 20 * dot_duration {
                    last_signal_change = now;
                    last_signal_state = false;
                    decoder.signal_event_end(false);
                    decoder.signal_event_end(true);
                    let mut msg = decoder.message.as_str().to_string();
                    msg = whitespace_regex.replace_all(&msg, " ").to_string();

                    if !msg.is_empty() {
                        crate::term::clear_screen();
                        for m in &message_log {
                            log_message(m);
                        }

                        let timestamp = Local::now().format("%y-%m-%d %H:%M:%S %p").to_string();
                        let mut m = Message {
                            timestamp: timestamp.clone(),
                            content: msg.clone(),
                        };
                        if output_morse {
                            m.content = text_to_morse(&m.content);
                        }
                        log_message(&m);
                        message_log.push(m);
                        decoder.message.clear();
                    }
                }
            }
            Err(err) if pcm.state() == State::XRun => {
                eprintln!("Overrun detected");
                pcm.prepare()?;
            }
            Err(err) => return Err(Box::new(err)),
        }

        thread::sleep(std::time::Duration::from_millis(10));
    }
}
