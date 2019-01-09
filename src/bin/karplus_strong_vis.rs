use patchwork::freeverb::Freeverb;

use rand::{Rng, thread_rng};

pub struct KarplusStrong {
    pub wavetable: Vec<f64>,
    pub index: usize,
    size: usize,
}

impl KarplusStrong {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        let p = (f64::from(sample_rate) / freq + 0.5) as usize;

        let mut wavetable = Vec::with_capacity(p);
        let mut rng = thread_rng();
        for _i in 0..p {
            // wavetable.push(rng.gen_range(-1.0, 1.0));
            if rng.gen_range(0.0, 1.0) > 0.5 {
                wavetable.push(1.0);
            } else {
                wavetable.push(-1.0);
            }
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

fn main() {
    let freq = 110.0;
    let sample_rate = 48_000;
    let mut ks = KarplusStrong::new(freq, sample_rate);

    let height = 48000;
    let width = ks.wavetable.len();

    // let mut freeverb = Freeverb::new();
    // freeverb.set_room_size(0.99);

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create("ks.wav", spec).unwrap();

    println!("P2");
    println!("{} {}", width, height);
    println!("255");

    for _ in 0..height {
        for i in 0..ks.wavetable.len() {
            let v = ks.wavetable[(i + ks.index) % ks.wavetable.len()];
            print!("{} ", ((v + 1.0) / 2.0 * 255.0).round());
        }
        println!();
        for _ in 0..1 {
            let v = ks.get();
            // let v = (ks.get() * std::i16::MAX as f64) as i32;
            // let (l, r) = freeverb.process((v, v));
            let (l, r) = (v, v);

            // // Distortion effect
            let l = if l > 0.0 {
               1.0 - f64::exp(-l)
            } else {
               -1.0 + f64::exp(l)
            };
            let r = if r > 0.0 {
               1.0 - f64::exp(-r)
            } else {
               -1.0 + f64::exp(r)
            };
            let l = (l * std::i16::MAX as f64) as i32;
            let r = (r * std::i16::MAX as f64) as i32;
            writer.write_sample(l).unwrap();
            writer.write_sample(r).unwrap();
        }
    }
}
