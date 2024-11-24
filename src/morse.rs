use rodio::{OutputStream, Sink, Source};
use std::time::Duration;

/// Custom audio source for generating tones
struct Tone {
    freq: f32,        // Frequency of the tone in Hz
    duration: u32,    // Duration of the tone in milliseconds
    sample_rate: u32, // Sample rate in Hz
    current_sample: u32,
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_sample >= (self.sample_rate * self.duration / 1000) {
            return None; // End of the tone
        }

        // Generate a sine wave
        let t = self.current_sample as f32 / self.sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * self.freq * t).sin();

        self.current_sample += 1;
        Some(sample)
    }
}

impl Source for Tone {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1 // Mono
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_millis(self.duration.into()))
    }
}

/// Converts words per minute (WPM) into a dot length in milliseconds
/// Based on standard Morse code timing where "PARIS" defines one word.
pub fn wpm_to_dot_length(wpm: u32) -> u32 {
    1200 / wpm
}

pub fn text_to_morse(text: &str) -> String {
    let morse_map = [
        ('A', ".-"),
        ('B', "-..."),
        ('C', "-.-."),
        ('D', "-.."),
        ('E', "."),
        ('F', "..-."),
        ('G', "--."),
        ('H', "...."),
        ('I', ".."),
        ('J', ".---"),
        ('K', "-.-"),
        ('L', ".-.."),
        ('M', "--"),
        ('N', "-."),
        ('O', "---"),
        ('P', ".--."),
        ('Q', "--.-"),
        ('R', ".-."),
        ('S', "..."),
        ('T', "-"),
        ('U', "..-"),
        ('V', "...-"),
        ('W', ".--"),
        ('X', "-..-"),
        ('Y', "-.--"),
        ('Z', "--.."),
        ('1', ".----"),
        ('2', "..---"),
        ('3', "...--"),
        ('4', "....-"),
        ('5', "....."),
        ('6', "-...."),
        ('7', "--..."),
        ('8', "---.."),
        ('9', "----."),
        ('0', "-----"),
        ('.', ".-.-.-"),
        (',', "--..--"),
        ('?', "..--.."),
        ('!', "-.-.--"),
        ('-', "-....-"),
        ('/', "-..-."),
        ('@', ".--.-."),
        ('(', "-.--."),
        (')', "-.--.-"),
    ];

    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter_map(|ch| {
                    morse_map.iter().find_map(|&(c, m)| {
                        if c == ch.to_ascii_uppercase() {
                            Some(m)
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<&str>>()
                .join(" ")
        })
        .collect::<Vec<String>>()
        .join("   ") // three spaces for word gaps
}

fn encode_morse(text: &str, dot_duration: u32, tone_freq: f32) -> Vec<(f32, u32)> {
    let dash_duration = dot_duration * 3; // Duration of a dash
    let char_gap_duration = dot_duration * 3; // Gap between characters
    let word_gap_duration = dot_duration * 7; // Gap between words

    let morse_code = text_to_morse(text);
    let morse_code = morse_code.replace("   ", "/").replace("  ", " ");

    let mut tones = Vec::new();

    for symbol in morse_code.chars() {
        match symbol {
            '.' => tones.push((tone_freq, dot_duration)),
            '-' => tones.push((tone_freq, dash_duration)),
            ' ' => tones.push((0.0, char_gap_duration)),
            '/' => tones.push((0.0, word_gap_duration)),
            _ => {}
        }
        tones.push((0.0, dot_duration)); // Gap between dots/dashes
    }

    tones
}

/// Morse code generator
fn play_morse_code(tones: Vec<(f32, u32)>, sink: &Sink) {
    let sample_rate = 44100;

    for (freq, duration) in tones {
        sink.append(Tone {
            freq,
            duration,
            sample_rate,
            current_sample: 0,
        });
    }
}

pub fn play(message: &str, dot_duration: u32, tone_freq: f32) {
    // Set up audio output
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Encode the message into Morse code
    let tones = encode_morse(message, dot_duration, tone_freq);

    // Play the Morse code
    play_morse_code(tones, &sink);

    // Keep the application alive until the sound finishes
    sink.sleep_until_end();
}
