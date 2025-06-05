use crate::{fft::compute_fft_magnitude, fft::hann_window, message::Message, morse::text_to_morse};
use alsa::pcm::{Access, Format, HwParams, State};
use alsa::{Direction, PCM};
use morse_codec::decoder::Decoder;
use regex::Regex;
use std::sync::mpsc::Sender;
use std::thread;

const DEBOUNCE_MS: u32 = 15;

pub fn listen_with_alsa(
    device_name: &str,
    tone_freq: f32,
    _bandwidth: f32,
    threshold: f32,
    dot_duration: u32,
    output_morse: bool,
    tx: Option<Sender<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let pcm = PCM::new(device_name, Direction::Capture, false)?;

    {
        let hwp = HwParams::any(&pcm)?;
        hwp.set_channels(1)?;
        hwp.set_rate(44100, alsa::ValueOr::Nearest)?;
        hwp.set_format(Format::s16())?;
        hwp.set_access(Access::RWInterleaved)?;
        pcm.hw_params(&hwp)?;
    }

    let io = pcm.io_i16()?;
    let sample_rate = 44100.0;
    let window_size = 1024;
    let mut buffer = Vec::with_capacity(window_size);
    let fft_window = hann_window(window_size);
    let target_bin = ((tone_freq / sample_rate) * window_size as f32).round() as usize;

    let mut decoder = Decoder::<9999>::new()
        .with_reference_short_ms(dot_duration as u16)
        .build();

    let whitespace_regex = Regex::new(r"\\s+").unwrap();
    let mut last_signal_state = false;
    let mut message_log = Vec::new();
    let mut sample_clock: usize = 0;
    let mut last_signal_change_sample: usize = 0;
    let mut read_buffer = vec![0i16; window_size];

    loop {
        match io.readi(&mut read_buffer) {
            Ok(read) => {
                for &sample in &read_buffer[..read] {
                    let s = sample as f32 / i16::MAX as f32;
                    buffer.push(s);
                    sample_clock += 1;

                    if buffer.len() >= window_size {
                        let magnitude =
                            compute_fft_magnitude(&buffer[..window_size], &fft_window, target_bin);
                        let tone_detected = magnitude > threshold;

                        let duration_samples = sample_clock - last_signal_change_sample;
                        let duration =
                            (duration_samples as f32 / sample_rate * 1000.0).round() as u32;

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
                                    timestamp: format!(
                                        "{:.3} sec",
                                        sample_clock as f32 / sample_rate
                                    ),
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
            }
            Err(_err) if pcm.state() == State::XRun => {
                eprintln!("Overrun detected");
                pcm.prepare()?;
            }
            Err(err) => return Err(Box::new(err)),
        }

        thread::sleep(std::time::Duration::from_millis(10));
    }
}
