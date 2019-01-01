use super::{Source, Phase};

const TWOPI: f64 = std::f64::consts::PI * 2.0;

#[derive(Debug, Clone)]
pub struct Sine {
    phase: Phase,
}

#[derive(Debug, Clone)]
pub struct Square {
    phase: Phase,
}

#[derive(Debug, Clone)]
pub struct Saw {
    phase: Phase,
}

impl Sine {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}

impl Square {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}

impl Saw {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}

impl Source for Sine {
    fn get(&mut self) -> f64 {
        (self.phase.get() * TWOPI).sin()
    }

    fn copy(&self) -> Box<Source> {
        Box::new(Sine { phase: self.phase.clone() })
    }
}

impl Source for Square {
    fn get(&mut self) -> f64 {
        if self.phase.get() < 0.5 {
            -1.0
        } else {
            1.0
        }
    }

    fn copy(&self) -> Box<Source> {
        Box::new(Square { phase: self.phase.clone() })
    }
}


impl Source for Saw {
    fn get(&mut self) -> f64 {
        self.phase.get()
    }

    fn copy(&self) -> Box<Source> {
        Box::new(Saw { phase: self.phase.clone() })
    }
}
