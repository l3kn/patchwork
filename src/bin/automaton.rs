extern crate hound;
extern crate rand;

use std::i16;

use rand::{Rng, thread_rng};

struct Cellular {
    cells: Vec<bool>,
    rule: u8,
}

impl Cellular {
    pub fn new(rule: u8) -> Self {
        let mut cells = vec![false; 16];
        let mut rng = thread_rng();

        for i in 0..16 {
            cells[i] = rng.gen();
        }

        Self { cells, rule }
    }

    pub fn step(&mut self) {
        let mut cells = vec![false; 16];

        for i in 0..16 {
            let left_i = (i + 15) % 16;
            let right_i = (i + 1) % 16;

            let mut code = 0;
            if self.cells[left_i] {
                code += 4;
            }
            if self.cells[i] {
                code += 2;
            }
            if self.cells[right_i] {
                code += 1;
            }

            let next = (self.rule & (1 << code)) != 0;
            cells[i] = next;
        }

        self.cells = cells;
    }

    pub fn print(&self) {
        for i in 0..16 {
            if self.cells[i] {
                print!("#");
            } else {
                print!("_");
            }
        }
        println!();
    }

    pub fn get_i16(&self) -> i16 {
        let mut v = 0;
        let mut bit = 1;
        for i in 0..16 {
            if self.cells[i] {
                v += bit;
            }
            bit <<= 1;
        }

        let max: f64 = 2.0_f64.powf(16.0) - 1.0;
        ((((v as f64) / max) * 2.0 - 1.0) * (std::i16::MAX as f64)) as i16
    }
}

fn main() {
    let mut c = Cellular::new(110);

    let sample_rate = 44100;
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create("sine.wav", spec).unwrap();
    for t in 0..(8 * 44100) {
        c.step();
        writer.write_sample(c.get_i16()).unwrap();
    }
}
