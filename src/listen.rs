use crate::morse::code_to_text;
use crate::prelude::*;
use audrey::read::Reader;

use std::path::Path;

/// Converts any supported audio format to a mono WAV file.
/// If the input file is already a mono WAV, it skips the conversion.
fn convert_to_mono_wav(
    input_path: &str,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let input_extension = Path::new(input_path)
        .extension()
        .and_then(|ext| ext.to_str());

    // Check if the file is already a mono WAV
    if input_extension == Some("wav") {
        let reader = Reader::open(input_path)?;
        let desc = reader.description();
        if desc.channel_count() == 1 {
            std::fs::copy(input_path, output_path)?;
            return Ok(());
        }
    }

    // Prepare output WAV writer
    let mut reader = Reader::open(input_path)?;
    let desc = reader.description();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: desc.sample_rate() as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(output_path, spec)?;

    // Read samples, convert to mono if needed, and write to the WAV file
    let mut samples_iter = reader.samples::<f32>();
    while let Some(sample) = samples_iter.next() {
        let sample = sample?;
        let mono_sample = if desc.channel_count() > 1 {
            sample / desc.channel_count() as f32 // Average channels
        } else {
            sample
        };

        let int_sample =
            (mono_sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        writer.write_sample(int_sample)?;
    }

    writer.finalize()?;
    Ok(())
}

pub fn listen(file_path: &str, tone_freq: f32, bandwidth: f32, threshold: f32, dot_duration: u32) {
    let temp_wav_path = "temp_mono.wav";

    debug!("Loading audio ...");
    match convert_to_mono_wav(file_path, temp_wav_path) {
        Ok(_) => {
            debug!("Analyzing audio ...");
            let morse_code = decode_morse_from_wav(
                &temp_wav_path,
                tone_freq,
                bandwidth,
                threshold,
                dot_duration,
            );
            println!("Decoded Morse Code: {}", morse_code);
        }
        Err(e) => eprintln!("Error converting audio: {}", e),
    }

    // Clean up the temporary WAV file
    if Path::new(temp_wav_path).exists() {
        let _ = std::fs::remove_file(temp_wav_path);
    }
}

use hound;
use rand::Rng;

fn fir_band_pass_filter(samples: &[f32], coefficients: &[f32]) -> Vec<f32> {
    let mut filtered_samples = vec![0.0; samples.len()];
    let filter_length = coefficients.len();

    for i in 0..samples.len() {
        let mut acc = 0.0;
        for j in 0..filter_length {
            if i >= j {
                acc += samples[i - j] * coefficients[j];
            }
        }
        filtered_samples[i] = acc;
    }

    filtered_samples
}

fn decode_morse_from_wav(
    file_path: &str,
    center_freq: f32,
    bandwidth: f32,
    threshold: f32,
    dot_duration_ms: u32,
) -> String {
    // Open the WAV file
    let mut reader = hound::WavReader::open(file_path).expect("Failed to open WAV file");
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    assert!(spec.channels == 1, "Only mono audio is supported");

    // Generate FIR filter coefficients for band-pass filtering
    let filter_order = 101;
    let nyquist = sample_rate as f32 / 2.0;
    let low_cutoff = (center_freq - bandwidth / 2.0) / nyquist;
    let high_cutoff = (center_freq + bandwidth / 2.0) / nyquist;

    let fir_coefficients = design_band_pass_filter(filter_order, low_cutoff, high_cutoff);

    // Read samples and normalize
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.expect("Invalid sample") as f32 / i16::MAX as f32)
        .collect();

    // Apply the band-pass filter
    let filtered_samples = fir_band_pass_filter(&samples, &fir_coefficients);

    let mut timeline = Vec::new();
    let mut current_duration = 0;
    let mut is_tone = false;
    let samples_per_dot = (sample_rate * dot_duration_ms / 1000) as usize;

    let window_span = 60; // Window span to select random samples from
    let sample_count = 10; // Number of samples to test within the window

    for start_index in (0..filtered_samples.len()).step_by(window_span) {
        let end_index = (start_index + window_span).min(filtered_samples.len());
        let mut rng = rand::thread_rng();

        // Randomly select `sample_count` samples within the window
        let random_samples: Vec<f32> = (0..sample_count)
            .filter_map(|_| {
                if end_index > start_index {
                    Some(filtered_samples[rng.gen_range(start_index..end_index)])
                } else {
                    None
                }
            })
            .collect();

        let tone_count = random_samples
            .iter()
            .filter(|&&s| s.abs() > threshold)
            .count();

        if tone_count as f32 >= 0.8 * sample_count as f32 {
            // Confirm tone
            if !is_tone {
                // Transition from silence to tone
                if current_duration > 0 {
                    let period = if current_duration > 7 * samples_per_dot {
                        "/"
                    } else if current_duration > 3 * samples_per_dot {
                        " "
                    } else {
                        ""
                    };
                    timeline.push(format!(
                        "{}: Silence for {} samples",
                        period, current_duration
                    ));
                }
                current_duration = 0;
                is_tone = true;
            }
            current_duration += 1;
        } else if tone_count as f32 <= 0.2 * sample_count as f32 {
            // Confirm silence
            if is_tone {
                // Transition from tone to silence
                if current_duration > 0 {
                    let period = if current_duration < samples_per_dot {
                        "."
                    } else {
                        "-"
                    };
                    timeline.push(format!("{}: Tone for {} samples", period, current_duration));
                }
                current_duration = 0;
                is_tone = false;
            }
            current_duration += 1;
        } else {
            current_duration += 1; // Continue in the current state
        }
    }

    for entry in timeline {
        println!("{}", entry);
    }

    // Return placeholder morse code (timeline replaces actual decoding for this implementation)
    "".to_string()
}

fn design_band_pass_filter(order: usize, low_cutoff: f32, high_cutoff: f32) -> Vec<f32> {
    use std::f32::consts::PI;
    let mut coefficients = vec![0.0; order];
    let mid = order / 2;
    for i in 0..order {
        if i == mid {
            coefficients[i] = 2.0 * (high_cutoff - low_cutoff);
        } else {
            let n = i as isize - mid as isize;
            let low = low_cutoff * (2.0 * PI * n as f32).sin() / (PI * n as f32);
            let high = high_cutoff * (2.0 * PI * n as f32).sin() / (PI * n as f32);
            coefficients[i] = high - low;
        }
        coefficients[i] *= 0.54 - 0.46 * (2.0 * PI * i as f32 / order as f32).cos();
        // Hamming window
    }
    coefficients
}
