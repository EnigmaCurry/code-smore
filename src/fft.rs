use num_complex::Complex;
use rustfft::FftPlanner;

pub fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|n| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * n as f32 / (size as f32 - 1.0)).cos()))
        .collect()
}

pub fn compute_fft_magnitude(input: &[f32], window: &[f32], bin: usize) -> f32 {
    let windowed: Vec<Complex<f32>> = input
        .iter()
        .zip(window.iter())
        .map(|(&x, &w)| Complex::new(x * w, 0.0))
        .collect();

    let mut spectrum = windowed.clone();
    FftPlanner::new()
        .plan_fft_forward(window.len())
        .process(&mut spectrum);

    spectrum[bin].norm() / (window.len() as f32 * 0.5)
}
