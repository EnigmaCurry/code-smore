use iir_filters::filter::{DirectForm2Transposed, Filter};
use iir_filters::filter_design::{butter, FilterType};
use iir_filters::sos::zpk2sos;

pub struct BandpassFilter {
    filter: DirectForm2Transposed,
}
impl BandpassFilter {
    /// Creates a new BandpassFilter with the given parameters.
    ///
    /// # Arguments
    /// * `order` - The order of the filter.
    /// * `tone_freq` - The center freq in Hz.
    /// * `bandwidth` - The filter bandwidth in Hz.
    /// * `sample_rate` - The sampling frequency in Hz.
    pub fn new(
        order: usize,
        tone_freq: f64,
        bandwidth: f64,
        sample_rate: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let cutoff_low = tone_freq - bandwidth / 2.0;
        let cutoff_high = tone_freq + bandwidth / 2.0;
        let zpk = butter(
            order as u32,
            FilterType::BandPass(cutoff_low, cutoff_high),
            sample_rate,
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
