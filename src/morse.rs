#![allow(unused_imports)]
use crate::prelude::*;
use anyhow;
use anyhow::Context;
use rodio::cpal::traits::HostTrait;
use rodio::DeviceTrait;
#[cfg(feature = "audio")]
use rodio::{OutputStream, Sink, Source};
#[cfg(feature = "gpio")]
use rppal;
#[cfg(feature = "audio")]
use serialport::SerialPort;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
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

/// RAII guard that asserts RTS on construction and de-asserts on drop.
#[cfg(feature = "audio")]
struct RtsGuard {
    port: Box<dyn SerialPort>,
}

#[cfg(feature = "audio")]
impl RtsGuard {
    pub fn new(port_name: &str) -> anyhow::Result<Self> {
        let mut port = serialport::new(port_name, 9_600)
            .timeout(Duration::from_millis(100))
            .open()
            .with_context(|| format!("opening serial port `{}`", port_name))?;
        port.write_request_to_send(true).context("asserting RTS")?;
        debug!("RTS ON");
        Ok(RtsGuard { port })
    }
}

#[cfg(feature = "audio")]
impl Drop for RtsGuard {
    fn drop(&mut self) {
        // best-effort deassert
        let _ = self.port.write_request_to_send(false);
        debug!("RTS OFF");
    }
}

/// Play a sequence of (frequency, duration_ms) via the given Sink.
/// If `ptt_rts_port` is set, RTS will be asserted slightly before playback
/// and deasserted slightly after playback finishes.
#[cfg(feature = "audio")]
pub fn play_morse_code(
    tones: Vec<(f32, u32)>,
    sink: &Sink,
    ptt_rts_port: Option<&str>,
    cw_rts_port: Option<&str>,
    rigctl_port: Option<&str>,
    rigctl_model: Option<&str>,
) -> anyhow::Result<()> {
    if let Some(cw_port) = cw_rts_port {
        // Send CW by asserting/deasserting RTS directly
        send_cw_rts(cw_port, &tones)?;
        return Ok(());
    }

    // Otherwise fall back to audio playback + PTT
    play_audio_with_ptt(&tones, sink, ptt_rts_port, rigctl_port, rigctl_model)
}

fn send_cw_rts(cw_port: &str, tones: &[(f32, u32)]) -> anyhow::Result<()> {
    use std::{thread, time::Duration};
    let mut port = serialport::new(cw_port, 9600)
        .timeout(Duration::from_millis(100))
        .open()
        .with_context(|| format!("opening CW RTS port {}", cw_port))?;

    for &(_freq, duration_ms) in tones {
        port.write_request_to_send(true)?; // key down
        thread::sleep(Duration::from_millis(duration_ms as u64));
        port.write_request_to_send(false)?; // key up
                                            // inter-element spacing can be added here if needed
    }

    Ok(())
}

#[cfg(feature = "audio")]
fn play_audio_with_ptt(
    tones: &[(f32, u32)],
    sink: &Sink,
    ptt_rts_port: Option<&str>,
    rigctl_port: Option<&str>,
    rigctl_model: Option<&str>,
) -> anyhow::Result<()> {
    let ptt_lead_in = Duration::from_millis(50);
    let ptt_hold_after = Duration::from_millis(50);

    let _rts = match ptt_rts_port {
        Some(port_name) => {
            let guard = RtsGuard::new(port_name)?;
            std::thread::sleep(ptt_lead_in);
            Some(guard)
        }
        None => None,
    };

    if let (Some(port), Some(model)) = (rigctl_port, rigctl_model) {
        let status = Command::new("rigctl")
            .arg("-m")
            .arg(model)
            .arg("-r")
            .arg(port)
            .arg("T")
            .arg("1")
            .status();

        if let Ok(s) = status {
            if !s.success() {
                return Err(anyhow::anyhow!("rigctl PTT on failed with status: {}", s));
            }
            std::thread::sleep(ptt_lead_in);
        } else {
            return Err(anyhow::anyhow!("Failed to spawn rigctl"));
        }
    }

    let sample_rate = 44_100;
    for (freq, duration) in tones {
        sink.append(Tone {
            freq: *freq,
            duration: *duration,
            sample_rate,
            current_sample: 0,
        });
    }

    sink.sleep_until_end();
    std::thread::sleep(ptt_hold_after);

    if let (Some(port), Some(model)) = (rigctl_port, rigctl_model) {
        let _ = Command::new("rigctl")
            .arg("-m")
            .arg(model)
            .arg("-r")
            .arg(port)
            .arg("T")
            .arg("0")
            .status();
    }

    Ok(())
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
    #[cfg(feature = "audio")]
    pub fn new_with_device(device_name: &str) -> anyhow::Result<Self> {
        let host = rodio::cpal::default_host();
        let device = host
            .output_devices()?
            .find(|d| d.name().map_or(false, |n| n == device_name))
            .ok_or_else(|| anyhow::anyhow!("Audio device not found: {}", device_name))?;

        let (stream, handle) = OutputStream::try_from_device(&device)?;
        Ok(Self {
            #[allow(clippy::arc_with_non_send_sync)]
            stream: Arc::new(stream),
            stream_handle: Arc::new(handle),
        })
    }

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
    pub fn play_gap(
        &self,
        dot_duration: u32,
        _ptt_rts_port: Option<&str>,
        _cw_rts_port: Option<&str>,
        _rigctl_port: Option<&str>,
        _rigctl_model: Option<&str>,
    ) {
        let tones = vec![(0.0, dot_duration)];
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        let _ = play_morse_code(tones, &sink, None, None, None, None);
        sink.sleep_until_end();
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_gap(
        &self,
        _dot_duration: u32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
    ) {
        error!("'audio' feature is disabled in this Cargo build. Program cannot play audio.");
    }

    #[cfg(feature = "audio")]
    pub fn play_nonblocking_tone(
        &self,
        dot_duration: u32,
        tone_freq: f32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
        rigctl_port: Option<&str>,
        rigctl_model: Option<&str>,
    ) {
        // clone the port name into an owned String so it can live in the 'static thread
        let owned_ptt_rts: Option<String> = ptt_rts_port.map(|s| s.to_string());
        let owned_cw_rts: Option<String> = cw_rts_port.map(|s| s.to_string());
        let owned_rigctl_port: Option<String> = rigctl_port.map(|s| s.to_string());
        let owned_rigctl_model: Option<String> = rigctl_model.map(|s| s.to_string());
        let stream_handle = self.stream_handle.clone();

        std::thread::spawn(move || {
            let tones = vec![(tone_freq, dot_duration)];
            let sink = Sink::try_new(&stream_handle).unwrap();
            play_morse_code(
                tones,
                &sink,
                owned_ptt_rts.as_deref(),
                owned_cw_rts.as_deref(),
                owned_rigctl_port.as_deref(),
                owned_rigctl_model.as_deref(),
            )
            .unwrap();
            sink.sleep_until_end();
        });
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_nonblocking_tone(
        &self,
        _dot_duration: u32,
        _tone_freq: f32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
    ) {
        error!("Error: Audio feature is disabled. Cannot play non-blocking tone.");
    }

    #[cfg(feature = "audio")]
    pub fn play_morse(
        &self,
        message: &str,
        dot_duration: u32,
        tone_freq: f32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
        rigctl_port: Option<&str>,
        rigctl_model: Option<&str>,
    ) {
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        let tones = morse_to_tones(message, dot_duration, tone_freq);
        let _ = play_morse_code(
            tones,
            &sink,
            ptt_rts_port,
            cw_rts_port,
            rigctl_port,
            rigctl_model,
        );
        sink.sleep_until_end();
    }

    #[cfg(feature = "audio")]
    pub fn play(
        &self,
        message: &str,
        dot_duration: u32,
        tone_freq: f32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
        rigctl_port: Option<&str>,
        rigctl_model: Option<&str>,
    ) {
        let sink = Sink::try_new(&self.stream_handle).unwrap();
        let tones = encode_morse(message, dot_duration, tone_freq);
        let _ = play_morse_code(
            tones,
            &sink,
            ptt_rts_port,
            cw_rts_port,
            rigctl_port,
            rigctl_model,
        );
        sink.sleep_until_end();
    }

    #[cfg(not(feature = "audio"))]
    pub fn play(
        &self,
        _message: &str,
        _dot_duration: u32,
        _tone_freq: f32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
    ) {
        error!("Error: Audio feature is disabled. Cannot play Morse code.");
    }

    #[cfg(not(feature = "audio"))]
    pub fn play_morse(
        &self,
        _message: &str,
        _dot_duration: u32,
        _tone_freq: f32,
        ptt_rts_port: Option<&str>,
        cw_rts_port: Option<&str>,
        rigctl_port: Option<&str>,
        rigctl_model: Option<&str>,
    ) {
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
