#[allow(unused_imports)]
use crate::filter::*;
#[allow(unused_imports)]
use crate::message::Message;
#[allow(unused_imports)]
use crate::morse::text_to_morse;
#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
use crate::pipewire::spa::pod::Pod;
#[allow(unused_imports)]
use crate::prelude::*;
#[allow(unused_imports)]
use chrono::Local;
#[allow(unused_imports)]
use morse_codec::decoder::Decoder;
#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
use pipewire as pw;
#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
use pw::properties::properties;
#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
use pw::{context::Context, main_loop::MainLoop, spa};
#[allow(unused_imports)]
use regex::Regex;
#[allow(unused_imports)]
use std::process::Command;
#[allow(unused_imports)]
use std::time::Instant;

#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
struct UserData {
    #[cfg(feature = "pipewire")]
    format: spa::param::audio::AudioInfoRaw,
    filter: Option<BandpassFilter>,
    message_log: Vec<Message>,
}

#[cfg(target_os = "windows")]
pub fn ensure_pipewire() {
    error!("Pipewire not enabled on windows");
}

#[cfg(target_os = "linux")]
#[cfg(not(feature = "pipewire"))]
pub fn ensure_pipewire() {
    error!("'pipewire' feature is disabled in the Cargo build. Program cannot receive audio.");
}

#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
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

#[cfg(target_os = "windows")]
pub fn listen(
    _tone_freq: f32,
    _bandwidth: f32,
    _threshold: f32,
    _dot_duration: u32,
    _output_morse: bool,
) -> Result<(), std::io::Error> {
    error!("listen feature not implemented on windows");
    return Ok(());
}

#[cfg(target_os = "linux")]
#[cfg(not(feature = "pipewire"))]
pub fn listen(
    _tone_freq: f32,
    _bandwidth: f32,
    _threshold: f32,
    _dot_duration: u32,
    _output_morse: bool,
) -> Result<(), std::io::Error> {
    error!("'pipewire' feature is disabled in the Cargo build. Program cannot receive audio.");
    return Ok(());
}

#[cfg(target_os = "linux")]
#[cfg(feature = "pipewire")]
pub fn listen(
    tone_freq: f32,
    bandwidth: f32,
    threshold: f32,
    dot_duration: u32,
    output_morse: bool,
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
                        let mut sum = 0.0_f32;
                        let mut count = 0;

                        let channel_samples: Vec<f64> = float_samples
                            .iter()
                            .skip(c.try_into().expect("Invalid skip in float_samples"))
                            .step_by(n_channels as usize)
                            .map(|&s| s as f64)
                            .collect();

                        let filtered_samples = channel_samples;

                        // Find peak and average sample
                        for &sample in &filtered_samples {
                            let abs_sample = sample.abs() as f32;
                            max = max.max(abs_sample);
                            sum += abs_sample;
                            count += 1;
                        }
                        let average = if count > 0 { sum / count as f32 } else { 0.0 } * 30.0;
                        //println!("{average}");
                        let tone_detected = average as f32 > threshold;
                        let timeout_duration = 20 * dot_duration;
                        let now = Instant::now();
                        let duration = now.duration_since(last_signal_change).as_millis() as u32;

                        // Detect message characters:
                        if tone_detected != last_signal_state {
                            decoder.signal_event(duration as u16, last_signal_state);

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

                                // Get the current timestamp
                                let timestamp =
                                    Local::now().format("%y-%m-%d %H:%M:%S %p").to_string();
                                // Print the new message and add it to the log
                                let mut m = Message {
                                    timestamp: timestamp.clone(),
                                    content: msg.clone(),
                                };
                                if output_morse {
                                    m.content = text_to_morse(&m.content);
                                }
                                println!("[{}] > {}", m.timestamp, m.content);

                                // Push the complete message into the log
                                user_data.message_log.push(m);
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
