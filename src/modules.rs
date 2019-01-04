use crate::util::{clamp, clamp_audio};
const TWOPI: f64 = std::f64::consts::PI * 2.0;

pub trait Module {
    fn get(&mut self) -> f64;
    fn set_input(&mut self, i: usize, val: f64);
}

#[derive(Debug, Clone)]
pub struct Phase {
    value: f64,
    step: f64,
    sample_rate: u32,
}

impl Phase {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self {
            value: 0.0,
            step: 1.0 / (sample_rate as f64 / freq),
            sample_rate,
        }
    }

    pub fn get(&mut self) -> f64 {
        self.value += self.step;
        if self.value > 1.0 {
            self.value -= 1.0;
        }
        self.value
    }

    pub fn set_freq(&mut self, freq: f64) {
        self.step = 1.0 / (self.sample_rate as f64 / freq);
    }
}

#[derive(Debug, Clone)]
pub struct Sine {
    phase: Phase,
}

impl Sine {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}

impl Module for Sine {
    fn get(&mut self) -> f64 {
        (self.phase.get() * TWOPI).sin()
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.phase.set_freq(val),
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Square0 {
    phase: Phase,
}
impl Square0 {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}
impl Module for Square0 {
    fn get(&mut self) -> f64 {
        if self.phase.get() > 0.5 {
            1.0
        } else {
            0.0
        }
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.phase.set_freq(val),
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Square {
    phase: Phase,
}
impl Square {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}
impl Module for Square {
    fn get(&mut self) -> f64 {
        if self.phase.get() > 0.5 {
            1.0
        } else {
            -1.0
        }
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.phase.set_freq(val),
            _ => ()
        }
    }
}


#[derive(Debug, Clone)]
pub struct Avg {
    v0: f64,
    v1: f64,
    res: f64,
}
impl Avg {
    pub fn new() -> Self {
        Self { v0: 0.0, v1: 0.0, res: 0.0 }
    }
}
impl Module for Avg {
    fn get(&mut self) -> f64 {
        self.res
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => {
                self.v0 = val;
                self.res = (self.v0 + self.v1) * 0.5;
            }
            1 => {
                self.v1 = val;
                self.res = (self.v0 + self.v1) * 0.5;
            }
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinMap {
    from: f64,
    range: f64,
    res: f64,
}

/// Map from -1..1 to from..to
impl LinMap {
    pub fn new(from: f64, to: f64) -> Self {
        Self {
            from,
            range: (to - from) * 0.5,
            res: 0.0
        }
    }

}
impl Module for LinMap {
    fn get(&mut self) -> f64 {
        self.res
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.res = self.from + (val + 1.0) * self.range,
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scale {
    factor: f64,
    res: f64,
}
impl Scale {
    pub fn new(factor: f64) -> Self {
        Self { factor, res: 0.0 }
    }
}
impl Module for Scale {
    fn get(&mut self) -> f64 {
        self.res
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.res = self.factor * val,
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mult {
    v0: f64,
    v1: f64,
    res: f64,
}
impl Mult {
    pub fn new() -> Self {
        Self { v0: 0.0, v1: 0.0, res: 0.0 }
    }
}
impl Module for Mult {
    fn get(&mut self) -> f64 {
        self.res
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => {
                self.v0 = val;
                self.res = self.v0 * self.v1;
            }
            1 => {
                self.v1 = val;
                self.res = self.v0 * self.v1;
            }
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Saw {
    phase: Phase,
}
impl Saw {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: Phase::new(freq, sample_rate) }
    }
}
impl Module for Saw {
    fn get(&mut self) -> f64 {
        self.phase.get() * 2.0 - 1.0
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.phase.set_freq(val),
            _ => ()
        }
    }
}

pub struct FeedbackDelay {
    buffer: Vec<f64>,
    gain: f64,
    size: usize,
    index: usize,
    input: f64,
}
impl FeedbackDelay {
    pub fn new(length: f64, gain: f64, sample_rate: u32) -> Self {
        let slots = (length * sample_rate as f64) as usize;
        let mut buffer = Vec::new();
        for _ in 0..slots {
            buffer.push(0.0);
        }

        Self { buffer, gain, size: slots, index: 0, input: 0.0 }
    }
}
impl Module for FeedbackDelay {
    fn get(&mut self) -> f64 {
        let write_index =
            if self.index == 0 {
                self.size - 1
            } else {
                self.index - 1
            };

        let res = self.buffer[self.index];
        let val = res * self.gain + self.input;
        self.buffer[write_index] = clamp_audio(val);

        self.index += 1;
        if self.index >= self.size {
            self.index = 0;
        }

        res
    }

    fn set_input(&mut self, i: usize, val: f64) {
        match i {
            0 => self.input = val,
            _ => ()
        }
    }
}

