use crate::filter::*;
use crate::message::Message;
use crate::pipewire::spa::pod::Pod;
use crate::prelude::*;
use crate::term::log_message;
use chrono::Local;
use morse_codec::decoder::Decoder;
use pipewire as pw;
use pw::properties::properties;
use pw::{context::Context, main_loop::MainLoop, spa};
use regex::Regex;
use std::process::Command;
use std::time::Instant;

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
    filter: Option<BandpassFilter>,
    message_log: Vec<Message>,
}

pub fn ensure_pipewire() {
    let service_status = Command::new("systemctl")
        .args(["--user", "is-active", "pipewire"])
        .output();

    match service_status {
        Ok(output) if output.status.success() => {
            //println!("PipeWire service is active");
        }
        _ => {
            eprintln!("The pipewire service is not active. Checking installation ...");
            let program_check = Command::new("pipewire").arg("--version").output();
            match program_check {
                Ok(output) if output.status.success() => {
                    eprintln!(
                        "pipewire is installed, but the service is not active. Please start it using: 'systemctl --user start pipewire'"
                    );
                }
                _ => {
                    eprintln!("pipewire is not installed. Please install it to proceed.");
                }
            }

            std::process::exit(1);
        }
    }
}

pub fn listen(
    tone_freq: f32,
    bandwidth: f32,
    threshold: f32,
    dot_duration: u32,
) -> Result<(), pipewire::Error> {
    pw::init();
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;

    let data = UserData {
        format: Default::default(),
        filter: None,
        message_log: Vec::new(),
    };

    let props = properties!(
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::STREAM_CAPTURE_SINK => "true"
    );

    let stream = pw::stream::Stream::new(&core, "audio-capture", props)?;

    let mut decoder = Decoder::<9999>::new()
        .with_reference_short_ms(dot_duration as u16)
        .build();
    let mut last_signal_change = Instant::now();
    let mut last_signal_state = false;
    let whitespace_regex = Regex::new(r"\s+").unwrap();

    clear_screen();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(move |_, user_data, id, param| {
            let Some(param) = param else {
                return;
            };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }
            user_data.format.parse(param).unwrap();
            user_data.filter = Some(
                BandpassFilter::new(
                    5,
                    tone_freq.into(),
                    bandwidth.into(),
                    user_data.format.rate() as f64,
                )
                .expect("expected filter"),
            );
        })
        .process(move |stream, user_data| match stream.dequeue_buffer() {
            None => println!("Out of buffers"),
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }

                let data = &mut datas[0];
                let n_channels = user_data.format.channels();
                if let Some(samples) = data.data() {
                    let float_samples: &mut [f32] = bytemuck::cast_slice_mut(samples);

                    for c in 0..n_channels {
                        let mut max: f32 = 0.0;

                        let channel_samples: Vec<f64> = float_samples
                            .iter()
                            .skip(c.try_into().expect("Invalid skip in float_samples"))
                            .step_by(n_channels as usize)
                            .map(|&s| s as f64)
                            .collect();

                        let filtered_samples = channel_samples;

                        for &sample in &filtered_samples {
                            max = max.max(sample.abs() as f32);
                        }

                        let peak = ((max * 30.0) as usize).clamp(0, 39);
                        let tone_detected = peak as f32 > threshold;
                        let timeout_duration = 20 * dot_duration;
                        let now = Instant::now();
                        let duration = now.duration_since(last_signal_change).as_millis() as u32;

                        // Detect message characters:
                        if tone_detected != last_signal_state {
                            decoder.signal_event(duration as u16, last_signal_state);
                            let mut msg = decoder.message.as_str().to_string();
                            msg = whitespace_regex.replace_all(&msg, " ").to_string();

                            if !msg.is_empty() {
                                clear_screen();
                                // Print all previous messages with timestamp
                                for logged_msg in &user_data.message_log {
                                    log_message(logged_msg);
                                }
                                // Print the current message as it is received:
                                println!("{msg}");
                            }

                            last_signal_change = now;
                            last_signal_state = tone_detected;
                        }

                        // Detect message end:
                        if duration > timeout_duration {
                            last_signal_change = now;
                            last_signal_state = false;
                            let mut msg = decoder.message.as_str().to_string();
                            msg = whitespace_regex.replace_all(&msg, " ").to_string();

                            if !msg.is_empty() {
                                decoder.signal_event_end(false);
                                decoder.signal_event_end(true);
                                msg = decoder.message.as_str().to_string();
                                msg = whitespace_regex.replace_all(&msg, " ").to_string();

                                clear_screen();
                                // Print all previous messages with timestamp
                                for logged_msg in &user_data.message_log {
                                    log_message(logged_msg);
                                }
                                // Get the current timestamp
                                let timestamp =
                                    Local::now().format("%y-%m-%d %H:%M:%S %p").to_string();
                                // Print the new message and add it to the log
                                let m = Message {
                                    timestamp: timestamp.clone(),
                                    content: msg.clone(),
                                };
                                log_message(&m);
                                // Push the complete message into the log
                                user_data.message_log.push(Message {
                                    timestamp,
                                    content: msg.clone(),
                                });
                                // Clear the decoder to prepare for a new message:
                                decoder.message.clear();
                            }
                        }
                    }
                }
            }
        })
        .register()?;

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    let obj = pw::spa::pod::Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: pw::spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .unwrap()
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    mainloop.run();
    Ok(())
}
