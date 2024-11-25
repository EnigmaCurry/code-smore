use crate::morse::code_to_text;
use audrey::read::Reader;
use hound::WavReader;
use std::fs::File;
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

/// Decodes Morse code from a .wav file.
/// Assumes the audio signal contains a consistent tone for dots and dashes keyed by hand.
fn decode_morse_from_wav(
    file_path: &str,
    threshold: f32,
) -> Result<(String, f32), Box<dyn std::error::Error>> {
    // Open the WAV file
    let mut reader = WavReader::open(file_path)?;

    // Ensure the WAV file is mono and has a valid sample rate
    let spec = reader.spec();
    if spec.channels != 1 {
        return Err("WAV file must be mono".into());
    }

    let sample_rate = spec.sample_rate;
    let mut samples: Vec<f32> = reader
        .samples::<i16>()
        .filter_map(Result::ok)
        .map(|s| s as f32 / i16::MAX as f32) // Normalize samples to [-1.0, 1.0]
        .collect();

    // Analyze the amplitude envelope to detect tone on/off events
    let mut signal_intervals = Vec::new();
    let mut in_signal = false;
    let mut signal_start = 0;

    for (i, &sample) in samples.iter().enumerate() {
        if sample.abs() > threshold {
            if !in_signal {
                in_signal = true;
                signal_start = i;
            }
        } else {
            if in_signal {
                in_signal = false;
                let signal_end = i;
                signal_intervals.push(signal_end - signal_start);
            }
        }
    }

    // Analyze intervals to determine dots, dashes, and spaces
    let mut morse_code = String::new();
    let mut average_dot_length = 0.0;
    if !signal_intervals.is_empty() {
        let dot_intervals: Vec<usize> = signal_intervals
            .iter()
            .filter(|&&interval| {
                let relative_length = interval as f32
                    / signal_intervals.iter().cloned().sum::<usize>() as f32
                    / signal_intervals.len() as f32;
                relative_length < 2.0
            })
            .cloned()
            .collect();

        if !dot_intervals.is_empty() {
            average_dot_length =
                dot_intervals.iter().sum::<usize>() as f32 / dot_intervals.len() as f32;
        }

        for interval in signal_intervals {
            let relative_length = interval as f32 / average_dot_length;
            if relative_length < 2.0 {
                morse_code.push('.'); // Dot
            } else if relative_length < 4.0 {
                morse_code.push('-'); // Dash
            } else {
                morse_code.push(' '); // Space between characters
            }
        }
    }

    // Translate Morse code to text
    let decoded_text = code_to_text(&morse_code);

    Ok((decoded_text, average_dot_length))
}

pub fn listen(file_path: &str, threshold: f32) {
    let temp_wav_path = "temp_mono.wav";

    match convert_to_mono_wav(file_path, temp_wav_path) {
        Ok(_) => match decode_morse_from_wav(temp_wav_path, threshold) {
            Ok((decoded, dot_length)) => {
                println!("Decoded text: {}", decoded);
                println!("Detected dot length: {} samples", dot_length);
            }
            Err(e) => eprintln!("Error decoding Morse code: {}", e),
        },
        Err(e) => eprintln!("Error converting audio: {}", e),
    }

    // Clean up the temporary WAV file
    if Path::new(temp_wav_path).exists() {
        let _ = std::fs::remove_file(temp_wav_path);
    }
}
