use crate::modules::Module;
use crate::SAMPLE_RATE;

use rand::{Rng, thread_rng};
use rand::rngs::ThreadRng;

pub struct KarplusStrong {
    pub wavetable: Vec<f64>,
    pub index: usize,
    size: usize,
    blend: f64,
    strech: f64,
    rng: ThreadRng,
}

impl Module for KarplusStrong {
    fn get(&mut self) -> f64 {
        let avg = 
            if self.rng.gen_bool(self.strech) {
                if self.rng.gen_bool(self.blend) {
                    (self.current() + self.prev()) * 0.5
                } else {
                    (self.current() + self.prev()) * -0.5
                }
            } else {
                if self.rng.gen_bool(self.blend) {
                    self.current() * 1.0
                } else {
                    self.current() * -1.0
                }
            };
        self.write(avg);

        self.index += 1;
        if self.index == self.size {
            self.index = 0;
        }

        avg
    }

    fn set_input(&mut self, i: usize, val: f64) {}
}

impl KarplusStrong {
    pub fn new(freq: f64, blend: f64, strech: f64) -> Self {
        let p = (SAMPLE_RATE / freq + 0.5) as usize;

        let mut wavetable = Vec::with_capacity(p);
        let mut rng = thread_rng();
        for _i in 0..p {
            // wavetable.push(rng.gen_range(-1.0, 1.0));
            if rng.gen_bool(0.5) {
                wavetable.push(1.0);
            } else {
                wavetable.push(-1.0);
            }
        }

        let strech = 1.0 / strech;
        Self { wavetable, size: p, index: 0, blend, strech, rng }
    }

    fn prev(&self) -> f64 {
        if self.index == 0 {
            self.wavetable[self.size - 1]
        } else {
            self.wavetable[self.index - 1]
        }
    }

    fn current(&self) -> f64 {
        self.wavetable[self.index]
    }

    fn write(&mut self, val: f64) {
        if self.index == 0 {
            self.wavetable[self.size - 1] = val;
        } else {
            self.wavetable[self.index - 1] = val;
        }
    }
}

