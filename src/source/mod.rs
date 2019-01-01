pub mod waves;
pub mod karplus_strong;
pub mod math;

pub trait Source {
    fn get(&mut self) -> f64;
    fn copy(&self) -> Box<Source>;
}

#[derive(Debug, Clone)]
pub struct Phase {
    value: f64,
    step: f64,
}

/// Sampled at `sample_rate`
/// this generates a ramp from 0 to 1 `freq` times per second
impl Phase {
    pub fn new(freq: f64, sample_rate: u32) -> Phase {
        Phase {
            value: 0.0,
            step: 1.0 / (sample_rate as f64 / freq)
        }
    }

    pub fn get(&mut self) -> f64 {
        self.value += self.step;
        if self.value > 1.0 {
            self.value -= 1.0;
        }
        self.value
    }
}
