#![allow(unused_imports)]
use crate::prelude::*;
#[cfg(feature = "audio")]
use rodio::{OutputStream, Sink, Source};
#[cfg(feature = "gpio")]
use rppal;
use std::collections::HashMap;
use std::sync::Arc;
#[allow(unused_imports)]
use std::thread::{self, sleep};
use std::time::Duration;

/// Custom audio source for generating tones
#[allow(dead_code)]
struct Tone {
    freq: f32,        // Frequency of the tone in Hz
    duration: u32,    // Duration of the tone in milliseconds
    sample_rate: u32, // Sample rate in Hz
    current_sample: u32,
}

impl Iterator for Tone {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let total_samples = self.sample_rate * self.duration / 1000;
        if self.current_sample >= total_samples {
            return None; // End of the tone
        }

        // Generate a sine wave
        let t = self.current_sample as f32 / self.sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * self.freq * t).sin();

        // Apply envelope (attack and release)
        let amplitude = if self.current_sample < (0.001 * self.sample_rate as f32) as u32 {
            // Attack phase
            self.current_sample as f32 / (0.001 * self.sample_rate as f32)
        } else if self.current_sample > total_samples - (0.001 * self.sample_rate as f32) as u32 {
            // Release phase
            (total_samples - self.current_sample) as f32 / (0.001 * self.sample_rate as f32)
        } else {
            // Sustain phase
            1.0
        };

        self.current_sample += 1;
        Some(sample * amplitude)
    }
}

#[cfg(feature = "audio")]
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

fn get_morse_maps() -> (HashMap<char, String>, HashMap<String, char>) {
    let forward_map = vec![
        ('A', ".-".to_string()),
        ('B', "-...".to_string()),
        ('C', "-.-.".to_string()),
        ('D', "-..".to_string()),
        ('E', ".".to_string()),
        ('F', "..-.".to_string()),
        ('G', "--.".to_string()),
        ('H', "....".to_string()),
        ('I', "..".to_string()),
        ('J', ".---".to_string()),
        ('K', "-.-".to_string()),
        ('L', ".-..".to_string()),
        ('M', "--".to_string()),
        ('N', "-.".to_string()),
        ('O', "---".to_string()),
        ('P', ".--.".to_string()),
        ('Q', "--.-".to_string()),
        ('R', ".-.".to_string()),
        ('S', "...".to_string()),
        ('T', "-".to_string()),
        ('U', "..-".to_string()),
        ('V', "...-".to_string()),
        ('W', ".--".to_string()),
        ('X', "-..-".to_string()),
        ('Y', "-.--".to_string()),
        ('Z', "--..".to_string()),
        ('1', ".----".to_string()),
        ('2', "..---".to_string()),
        ('3', "...--".to_string()),
        ('4', "....-".to_string()),
        ('5', ".....".to_string()),
        ('6', "-....".to_string()),
        ('7', "--...".to_string()),
        ('8', "---..".to_string()),
        ('9', "----.".to_string()),
        ('0', "-----".to_string()),
        ('.', ".-.-.-".to_string()),
        (',', "--..--".to_string()),
        ('?', "..--..".to_string()),
        ('!', "-.-.--".to_string()),
        ('-', "-....-".to_string()),
        ('/', "-..-.".to_string()),
        ('@', ".--.-.".to_string()),
        ('(', "-.--.".to_string()),
        (')', "-.--.-".to_string()),
    ];

    let mut forward_hashmap = HashMap::new();
    let mut reverse_hashmap = HashMap::new();

    for (ch, code) in forward_map {
        forward_hashmap.insert(ch, code.clone());
        reverse_hashmap.insert(code, ch);
    }
    (forward_hashmap, reverse_hashmap)
}

pub fn code_to_text(code: &str) -> String {
    let morse_map = get_morse_maps().1;
    regex::Regex::new(r"\s{3,}") // Match three or more spaces
        .unwrap()
        .replace_all(code, " / ")
        .split(" / ") // Split by word gaps
        .map(|word| {
            word.split_whitespace() // Split by character gaps
                .filter_map(|morse| morse_map.get(morse)) // Lookup each Morse code
                .collect::<String>() // Collect decoded characters into a string (word)
        })
        .collect::<Vec<String>>() // Collect words into a vector
        .join(" ") // Join words with spaces
}

pub fn text_to_morse(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter_map(|ch| {
                    get_morse_maps().0.iter().find_map(|(c, m)| {
                        if *c == ch.to_ascii_uppercase() {
                            Some(m.clone())
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<String>>()
                .join(" ")
        })
        .collect::<Vec<String>>()
        .join(" / ") // word gap
}

#[allow(dead_code)]
fn encode_morse(text: &str, dot_duration: u32, tone_freq: f32) -> Vec<(f32, u32)> {
    let morse_code = text_to_morse(text);
    let morse_code = regex::Regex::new(r"\s{3,}") // Match three or more spaces
        .unwrap()
        .replace_all(&morse_code, "/")
        .to_string();
    let morse_code = regex::Regex::new(r"\s{2}") // Match exactly two spaces
        .unwrap()
        .replace_all(&morse_code, " ")
        .to_string();

    morse_to_tones(&morse_code, dot_duration, tone_freq)
}

#[allow(dead_code)]
fn morse_to_tones(morse_code: &str, dot_duration: u32, tone_freq: f32) -> Vec<(f32, u32)> {
    let dash_duration = dot_duration * 3; // Duration of a dash
    let char_gap_duration = dot_duration * 3; // Gap between characters
    let word_gap_duration = dot_duration * 7; // Gap between words

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
#[cfg(feature = "audio")]
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

#[cfg(feature = "gpio")]
fn gpio_morse_code(tones: Vec<(f32, u32)>, pin_number: u8) {
    let mut pin = rppal::gpio::Gpio::new()
        .expect("Failed to access GPIO")
        .get(pin_number)
        .expect("Failed to get GPIO pin")
        .into_output();
    for (frequency, duration) in tones {
        if frequency == 0. || duration == 0 {
            //info!("gap: d: {duration}");
            pin.set_low();
        } else {
            //info!("f: {frequency} d: {duration}");
            pin.set_high();
        }
        sleep(Duration::from_millis(duration.into()));
    }
    pin.set_low();
}

pub struct MorsePlayer {
    #[cfg(feature = "audio")]
    #[allow(dead_code)]
    stream: Arc<OutputStream>, // Keep the stream alive

    #[cfg(feature = "audio")]
    stream_handle: Arc<rodio::OutputStreamHandle>, // Shareable stream handle
}

impl MorsePlayer {
    pub fn new() -> Self {
        #[cfg(feature = "audio")]
        {
            // Set up the audio output once
            let stream = OutputStream::try_default().unwrap();
            let stream_handle = Arc::new(stream.1);

            return Self {
                #[allow(clippy::arc_with_non_send_sync)]
                stream: Arc::new(stream.0),
                stream_handle,
            };
        }

        #[cfg(not(feature = "audio"))]
        {
            Self {}
        }
    }

    #[cfg(feature = "audio")]
    pub fn play_gap(&self, dot_duration: u32) {
        let tones = vec![(0.0, dot_duration)];
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        play_morse_code(tones, &sink);
        sink.sleep_until_end();
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_gap(&self, _dot_duration: u32) {
        error!("'audio' feature is disabled in this Cargo build. Program cannot play audio.");
    }

    #[cfg(feature = "audio")]
    pub fn play_nonblocking_tone(&self, dot_duration: u32, tone_freq: f32) {
        let stream_handle = self.stream_handle.clone();
        std::thread::spawn(move || {
            let tones = vec![(tone_freq, dot_duration)];
            let sink = Sink::try_new(&stream_handle).unwrap();
            play_morse_code(tones, &sink);
            sink.sleep_until_end(); // Blocks within this thread, not the main one
        });
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_nonblocking_tone(&self, _dot_duration: u32, _tone_freq: f32) {
        error!("Error: Audio feature is disabled. Cannot play non-blocking tone.");
    }

    #[cfg(feature = "audio")]
    pub fn play_morse(&self, message: &str, dot_duration: u32, tone_freq: f32) {
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        let tones = morse_to_tones(message, dot_duration, tone_freq);
        play_morse_code(tones, &sink);
        sink.sleep_until_end();
    }

    #[cfg(feature = "audio")]
    pub fn play(&self, message: &str, dot_duration: u32, tone_freq: f32) {
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        let tones = encode_morse(message, dot_duration, tone_freq);
        play_morse_code(tones, &sink);
        sink.sleep_until_end();
    }

    #[cfg(not(feature = "audio"))]
    pub fn play(&self, _message: &str, _dot_duration: u32, _tone_freq: f32) {
        error!("Error: Audio feature is disabled. Cannot play Morse code.");
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_morse(&self, _message: &str, _dot_duration: u32, _tone_freq: f32) {
        error!("Error: Audio feature is disabled. Cannot play Morse code.");
    }

    #[cfg(feature = "gpio")]
    pub fn gpio_morse(&self, message: &str, dot_duration: u32, pin_number: u8) {
        let tones = morse_to_tones(message, dot_duration, 333.); //frequncy is unused but must be >0
        gpio_morse_code(tones, pin_number);
    }

    #[cfg(feature = "gpio")]
    pub fn gpio(&self, message: &str, dot_duration: u32, pin_number: u8) {
        let tones = encode_morse(message, dot_duration, 333.); //frequency is unused but must be >0
        gpio_morse_code(tones, pin_number);
    }

    #[cfg(not(feature = "gpio"))]
    pub fn gpio_morse(&self, _message: &str, _dot_duration: u32, _gpio_pin: u8) {
        error!("Error: GPIO feature is disabled. Cannot play Morse code via GPIO.");
    }

    #[cfg(not(feature = "gpio"))]
    pub fn gpio(&self, _message: &str, _dot_duration: u32, _gpio_pin: u8) {
        error!("Error: GPIO feature is disabled. Cannot perform GPIO operations.");
    }

    #[cfg(feature = "gpio")]
    pub fn gpio_gap(&self, dot_duration: u32, pin_number: u8) {
        let mut pin = rppal::gpio::Gpio::new()
            .expect("Failed to access GPIO")
            .get(pin_number)
            .expect("Failed to get GPIO pin")
            .into_output();
        pin.set_low();
        sleep(Duration::from_millis(dot_duration.into()));
    }

    #[cfg(not(feature = "gpio"))]
    pub fn gpio_gap(&self, _dot_duration: u32, _gpio_pin: u8) {
        error!("Error: GPIO feature is disabled. Cannot perform GPIO gap.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_to_morse() {
        assert_eq!(text_to_morse("SOS"), "... --- ...");
        assert_eq!(
            text_to_morse("Hello   World 123. How are you?"),
            ".... . .-.. .-.. --- / .-- --- .-. .-.. -.. / .---- ..--- ...-- .-.-.- / .... --- .-- / .- .-. . / -.-- --- ..- ..--.."
        );
    }
}
