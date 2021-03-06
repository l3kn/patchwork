use rand::{Rng, thread_rng};

use crate::SAMPLE_RATE;
use super::Source;

pub struct KarplusStrong {
    wavetable: Vec<f64>,
    index: usize,
    size: usize,
}

impl KarplusStrong {
    pub fn new(freq: f64) -> Self {
        let p = (SAMPLE_RATE / freq + 0.5) as usize;

        let mut wavetable = Vec::with_capacity(p);
        let mut rng = thread_rng();
        for _i in 0..p {
            wavetable.push(rng.gen_range(-1.0, 1.0));
            // if rng.gen_range(0.0, 1.0) > 0.5 {
            //     wavetable.push(1.0);
            // } else {
            //     wavetable.push(-1.0);
            // }
            // if ((i * 4) % p) > (p / 2) {
            //     wavetable.push(rng.gen_range(-1.0, -0.5));
            // } else {
            //     wavetable.push(rng.gen_range(0.5, 1.0));
            // }
            // wavetable.push((((i as f64) / p as f64) % 1.0) * 2.0 - 1.0);
            // wavetable.push(rng.gen_range(-1.0, 1.0));
        }


        Self { wavetable, size: p, index: 0 }
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

impl Source for KarplusStrong {
    fn get(&mut self) -> f64 {
        let avg = (self.current() + self.prev()) * 0.5;
        self.write(avg);

        self.index += 1;
        if self.index == self.size {
            self.index = 0;
        }

        avg
    }
}
