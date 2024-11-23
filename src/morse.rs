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

fn encode_morse(text: &str, dot_duration: u32, tone_freq: f32) -> Vec<(f32, u32)> {
    // dot_duration is duration of a dot in milliseconds
    let dash_duration = dot_duration * 3; // Duration of a dash
    let char_gap_duration = dot_duration * 3; // Gap between characters
    let word_gap_duration = dot_duration * 7; // Gap between words

    // Morse code mapping
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

    let mut tones = Vec::new();

    for word in text.split_whitespace() {
        for ch in word.chars() {
            if let Some(morse) = morse_map.iter().find_map(|&(c, m)| {
                if c == ch.to_ascii_uppercase() {
                    Some(m)
                } else {
                    None
                }
            }) {
                for symbol in morse.chars() {
                    let duration = match symbol {
                        '.' => dot_duration,
                        '-' => dash_duration,
                        _ => continue,
                    };
                    tones.push((tone_freq, duration));
                    tones.push((0.0, dot_duration)); // Gap between dots/dashes
                }
                tones.push((0.0, char_gap_duration)); // Gap between characters
            }
        }
        tones.push((0.0, word_gap_duration)); // Gap between words
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
