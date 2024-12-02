use crate::pipewire::spa::pod::Pod;
use crate::prelude::*;
use pipewire as pw;
use pw::properties::properties;
use pw::{context::Context, main_loop::MainLoop, spa};
use std::mem;

use iir_filters::filter::{DirectForm2Transposed, Filter};
use iir_filters::filter_design::{butter, FilterType};
use iir_filters::sos::zpk2sos;

struct BandpassFilter {
    filter: DirectForm2Transposed,
}
impl BandpassFilter {
    /// Creates a new BandpassFilter with the given parameters.
    ///
    /// # Arguments
    /// * `order` - The order of the filter.
    /// * `cutoff_low` - The lower cutoff frequency in Hz.
    /// * `cutoff_hi` - The upper cutoff frequency in Hz.
    /// * `fs` - The sampling frequency in Hz.
    pub fn new(
        order: usize,
        cutoff_low: f64,
        cutoff_hi: f64,
        fs: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let zpk = butter(
            order as u32,
            FilterType::BandPass(cutoff_low, cutoff_hi),
            fs,
        )?;
        let sos = zpk2sos(&zpk, None)?;
        let filter = DirectForm2Transposed::new(&sos);

        Ok(Self { filter })
    }

    /// Applies the bandpass filter to an input signal.
    ///
    /// # Arguments
    /// * `input` - A slice of input signal samples.
    ///
    /// # Returns
    /// A `Vec<f64>` containing the filtered signal.
    pub fn apply(&mut self, input: &[f64]) -> Vec<f64> {
        input.iter().map(|&x| self.filter.filter(x)).collect()
    }
}

struct UserData {
    format: spa::param::audio::AudioInfoRaw,
    cursor_move: bool,
    filter: Option<BandpassFilter>,
}

pub fn main() -> Result<(), pipewire::Error> {
    // Initialization code...
    pw::init();
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = core.get_registry().expect("Invalid pipewire registry");

    let data = UserData {
        format: Default::default(),
        cursor_move: false,
        filter: None,
    };

    let props = properties!(
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Communication",
        *pw::keys::STREAM_CAPTURE_SINK => "true"
    );

    // Initialize the stream with the core and properties
    let stream = pw::stream::Stream::new(&core, "audio-capture", props)?;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(|_, user_data, id, param| {
            // Handle format changes...
            // NULL means to clear the format
            let Some(param) = param else {
                return;
            };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }
            user_data.format.parse(param).unwrap();
            user_data.filter = Some(
                BandpassFilter::new(5, 500.0, 1000.0, user_data.format.rate() as f64)
                    .expect("expected filter"),
            );
        })
        .process(|stream, user_data| match stream.dequeue_buffer() {
            None => println!("Out of buffers"),
            Some(mut buffer) => {
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }

                let data = &mut datas[0];
                let n_channels = user_data.format.channels();
                let n_samples = data.chunk().size() / (mem::size_of::<f32>() as u32);

                if let Some(samples) = data.data() {
                    // Interpret the buffer as f32 samples
                    let float_samples: &mut [f32] = bytemuck::cast_slice_mut(samples);

                    if user_data.cursor_move {
                        // Move the cursor up for clean output
                        print!("\x1B[{}A", n_channels + 1);
                    }
                    println!();

                    for c in 0..n_channels {
                        let mut max: f32 = 0.0;

                        // Extract the samples for the current channel
                        let channel_samples: Vec<f32> = float_samples
                            .iter()
                            .skip(c.try_into().expect("Invalid skip in float_samples"))
                            .step_by(n_channels as usize)
                            .cloned()
                            .collect();

                        // Convert Vec<f32> to Vec<f64>
                        let channel_samples_f64: Vec<f64> =
                            channel_samples.iter().map(|&s| s as f64).collect();

                        // Apply the bandpass filter
                        let filtered_samples = if let Some(filter) = &mut user_data.filter {
                            filter.apply(&channel_samples_f64)
                        } else {
                            channel_samples_f64
                        };

                        // Convert Vec<f64> back to Vec<f32>
                        let filtered_samples_f32: Vec<f32> =
                            filtered_samples.iter().map(|&s| s as f32).collect();

                        // Write the filtered samples back to the buffer
                        for (i, &sample) in filtered_samples_f32.iter().enumerate() {
                            let idx = i * n_channels as usize + c as usize;
                            float_samples[idx] = sample;
                            max = max.max(sample.abs());
                        }

                        // Visualize the peak
                        let peak = ((max * 30.0) as usize).clamp(0, 39);
                        println!(
                            "channel {}: |{:>w1$}{:w2$}| peak: {:.3}",
                            c,
                            "*",
                            "",
                            max,
                            w1 = peak + 1,
                            w2 = 40 - peak
                        );
                    }

                    user_data.cursor_move = true;
                }
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
    info!("well");
    // and wait while we let things run
    mainloop.run();
    Ok(())
}
