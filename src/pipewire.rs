use crate::filter::*;
use crate::pipewire::spa::pod::Pod;
use crate::prelude::*;
use morse_codec::decoder::Decoder;
use pipewire as pw;
use pw::properties::properties;
use pw::{context::Context, main_loop::MainLoop, spa};
use std::fmt::Write;
use std::time::Instant;

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
    filter: Option<BandpassFilter>,
    cursor_move: bool,
}

pub fn listen(
    tone_freq: f32,
    bandwidth: f32,
    threshold: f32,
    dot_duration: u32,
) -> Result<(), pipewire::Error> {
    // Initialization code...
    pw::init();
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    //let registry = core.get_registry().expect("Invalid pipewire registry");

    let data = UserData {
        format: Default::default(),
        filter: None,
        cursor_move: false,
    };

    let props = properties!(
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::STREAM_CAPTURE_SINK => "true"
    );

    // Initialize the stream with the core and properties
    let stream = pw::stream::Stream::new(&core, "audio-capture", props)?;

    // Morse decoder:
    let mut decoder = Decoder::<9999>::new()
        .with_reference_short_ms(dot_duration as u16)
        .build();
    let mut last_signal_change = Instant::now();
    let mut last_signal_state = false;

    clear_screen();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(move |_, user_data, id, param| {
            // Handle format changes...
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
                    if user_data.cursor_move {
                        print!("\x1B[{}A", 1);
                    }
                    // Interpret the buffer as f32 samples
                    let float_samples: &mut [f32] = bytemuck::cast_slice_mut(samples);

                    for c in 0..n_channels {
                        let mut max: f32 = 0.0;

                        // Extract and filter the samples
                        let channel_samples: Vec<f64> = float_samples
                            .iter()
                            .skip(c.try_into().expect("Invalid skip in float_samples"))
                            .step_by(n_channels as usize)
                            .map(|&s| s as f64)
                            .collect();

                        // let filtered_samples = if let Some(filter) = &mut user_data.filter {
                        //     filter.apply(&channel_samples)
                        // } else {
                        //     channel_samples
                        // };

                        let filtered_samples = channel_samples;

                        // Determine if tone is detected
                        for &sample in &filtered_samples {
                            max = max.max(sample.abs() as f32);
                        }

                        let peak = ((max * 30.0) as usize).clamp(0, 39);
                        let tone_detected = peak as f32 > threshold;

                        // Handle signal state changes
                        let now = Instant::now();
                        let duration = now.duration_since(last_signal_change).as_millis() as u32;

                        let mut current_line = String::new();
                        let mut current_line_length = 0;

                        if tone_detected != last_signal_state {
                            // Send the signal event to the decoder
                            decoder.signal_event(duration as u16, last_signal_state);
                            last_signal_change = now;
                            last_signal_state = tone_detected;

                            // Check if a new character is decoded
                            if !decoder.message.is_empty() {
                                println!("Decoded message: {}", decoder.message.as_str());
                                println!("");
                            }
                        }
                    }
                }
                user_data.cursor_move = true;
            }
        })
        .register()?;

    /* Make one parameter with the supported formats. The SPA_PARAM_EnumFormat
     * id means that this is a format enumeration (of 1 value).
     * We leave the channels and rate empty to accept the native graph
     * rate and channels. */
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

    /* Now connect this stream. We ask that our process function is
     * called in a realtime thread. */
    stream.connect(
        spa::utils::Direction::Input,
        None,
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    // and wait while we let things run
    mainloop.run();
    Ok(())
}
