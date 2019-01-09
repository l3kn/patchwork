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
pub struct Square0 {
    phase: Phase,
}

#[derive(Debug, Clone)]
pub struct Saw {
    phase: Phase,
}

impl Sine {
    pub fn new(freq: f64) -> Self {
        Self { phase: Phase::new(freq) }
    }
}

impl Square {
    pub fn new(freq: f64) -> Self {
        Self { phase: Phase::new(freq) }
    }
}

impl Square0 {
    pub fn new(freq: f64) -> Self {
        Self { phase: Phase::new(freq) }
    }
}

impl Saw {
    pub fn new(freq: f64) -> Self {
        Self { phase: Phase::new(freq) }
    }
}

impl Source for Sine {
    fn get(&mut self) -> f64 {
        (self.phase.get() * TWOPI).sin()
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
}

impl Source for Square0 {
    fn get(&mut self) -> f64 {
        if self.phase.get() < 0.5 {
            0.0
        } else {
            1.0
        }
    }
}

impl Source for Saw {
    fn get(&mut self) -> f64 {
        self.phase.get() * 2.0 - 1.0
    }
}
