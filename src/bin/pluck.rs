extern crate hound;
extern crate rand;

use std::i16;

use rand::thread_rng;

struct KarplusStrong {
    wavetable: Vec<f32>,
    index: usize,
    size: usize,
}

#[derive(Clone, Copy)]
enum Note {
    C(i32),
    CS(i32),
    D(i32),
    DS(i32),
    E(i32),
    F(i32),
    FS(i32),
    G(i32),
    GS(i32),
    A(i32),
    AS(i32),
    B(i32),
} 

impl Note {
    pub fn freq(&self) -> f64 {
        match *self {
            Note::C(o)  => 16.35160 * f64::powi(2.0, o),
            Note::CS(o) => 17.32391 * f64::powi(2.0, o),
            Note::D(o)  => 18.35405 * f64::powi(2.0, o),
            Note::DS(o) => 19.44544 * f64::powi(2.0, o),
            Note::E(o)  => 20.60172 * f64::powi(2.0, o),
            Note::F(o)  => 21.82676 * f64::powi(2.0, o),
            Note::FS(o) => 23.12465 * f64::powi(2.0, o),
            Note::G(o)  => 24.49971 * f64::powi(2.0, o),
            Note::GS(o) => 25.95654 * f64::powi(2.0, o),
            Note::A(o)  => 27.50000 * f64::powi(2.0, o),
            Note::AS(o) => 29.13524 * f64::powi(2.0, o),
            Note::B(o)  => 30.86771 * f64::powi(2.0, o),
        }
    }
}

impl KarplusStrong {
    fn new(note: Note, sample_rate: usize) -> Self {
        let p = (sample_rate as f64 / note.freq() + 0.5) as usize;

        let mut wavetable = Vec::with_capacity(p);
        let mut rng = thread_rng();
        for i in 0..p {
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
            wavetable.push((((i as f32) / p as f32) % 1.0) * 2.0 - 1.0);
            // wavetable.push(rng.gen_range(-1.0, 1.0));
        }


        let mut ks = Self { wavetable, size: p, index: 0 };

        for _ in 0..p {
            ks.sample();
            ks.sample();
            // ks.sample();
        }

        ks
    }

    fn prev(&self) -> f32 {
        if self.index == 0 {
            self.wavetable[self.size - 1]
        } else {
            self.wavetable[self.index - 1]
        }
    }

    fn current(&self) -> f32 {
        self.wavetable[self.index]
    }

    fn write(&mut self, val: f32) {
        if self.index == 0 {
            self.wavetable[self.size - 1] = val;
        } else {
            self.wavetable[self.index - 1] = val;
        }
    }

    fn sample(&mut self) -> f32 {
        let avg = (self.current() + self.prev()) * 0.497;
        self.write(avg);

        self.index += 1;
        if self.index == self.size {
            self.index = 0;
        }

        avg
    }
}

const FRETS: [[Note; 6]; 13] = [
    [Note::E(2), Note::A(2), Note::D(3), Note::G(3), Note::B(3), Note::E(4)],
    [Note::F(2), Note::AS(2), Note::DS(3), Note::GS(3), Note::C(4), Note::F(4)],
    [Note::FS(2), Note::B(2), Note::E(3), Note::A(3), Note::CS(4), Note::FS(4)],
    [Note::G(2), Note::C(3), Note::F(3), Note::AS(3), Note::D(4), Note::G(4)],
    [Note::GS(2), Note::CS(3), Note::FS(3), Note::B(3), Note::DS(4), Note::GS(4)],
    [Note::A(2), Note::D(3), Note::G(3), Note::C(4), Note::E(4), Note::A(4)],
    [Note::AS(2), Note::DS(3), Note::GS(3), Note::CS(4), Note::F(4), Note::AS(4)],
    [Note::B(2), Note::E(3), Note::A(3), Note::D(4), Note::FS(4), Note::B(4)],
    [Note::C(3), Note::F(3), Note::AS(3), Note::DS(4), Note::G(4), Note::C(4)],
    [Note::CS(3), Note::FS(3), Note::B(3), Note::E(4), Note::GS(4), Note::CS(4)],
    [Note::D(3), Note::G(3), Note::C(4), Note::F(4), Note::A(4), Note::D(4)],
    [Note::DS(3), Note::GS(3), Note::CS(4), Note::FS(4), Note::AS(4), Note::DS(4)],
    [Note::E(3), Note::A(3), Note::D(4), Note::G(4), Note::B(4), Note::E(4)],
];

struct Chord {
    notes: Vec<KarplusStrong>,
    index: usize,
}

impl Chord {
    fn new(desc: [isize; 6], sample_rate: usize) -> Self {
        let mut notes = Vec::new();

        for i in 0..6 {
            if desc[i] >= 0 {
                let n = FRETS[desc[i] as usize][i];
                notes.push(KarplusStrong::new(n, sample_rate));
            }
        }

        Self { notes, index: 0 }
    }

    fn sample(&mut self) -> f32 {
        let mut res = 0.0;
        for i in 0..self.notes.len() {
            if self.index >= i * 2000 {
                res += self.notes[i].sample();
            }
        }
        self.index += 1;
        res / self.notes.len() as f32
    }
}


fn main() {
    let sample_rate = 44100;
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut chords = Vec::new();
    // chords.push(Chord::new([-1, 0, 3, 3, 3, 0], sample_rate as usize));
    // chords.push(Chord::new([0, 3, 3, 2, 0, 0], sample_rate as usize));
    // A major 
    // chords.push(Chord::new([-1, 0, 2, 2, 2, 0], sample_rate as usize));
    // A minor
    // chords.push(Chord::new([-1, 0, 2, 2, 1, 0], sample_rate as usize));
    // B major
    // chords.push(Chord::new([-1, -1, 4, 4, 4, 2], sample_rate as usize));
    // B minor
    // chords.push(Chord::new([-1, 2, 4, 4, 3, 2], sample_rate as usize));

    // D minor
    chords.push(Chord::new([-1, -1, 0, 2, 3, 1], sample_rate as usize));

    // A sharp
    chords.push(Chord::new([-1, -1, 8, 10, 11, 10], sample_rate as usize));

    // F
    chords.push(Chord::new([-1, -1, 3, 5, 6, 5], sample_rate as usize));

    // G minor
    chords.push(Chord::new([-1, -1, 5, 7, 8, 6], sample_rate as usize));

    // D minor
    chords.push(Chord::new([-1, -1, 0, 2, 3, 1], sample_rate as usize));

    // A sharp
    chords.push(Chord::new([-1, -1, 8, 10, 11, 10], sample_rate as usize));

    // F
    chords.push(Chord::new([-1, -1, 3, 5, 6, 5], sample_rate as usize));

    // G minor
    chords.push(Chord::new([-1, -1, 5, 7, 8, 6], sample_rate as usize));

    let sample_rate = 44100;
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create("sine.wav", spec).unwrap();
    for t in 0..(8 * 44100) {
        let mut sample = 0.0;
        for i in 0..chords.len() {
            if t >= i * 44000 {
                sample += chords[i].sample();
            }
        }
        let amplitude = i16::MAX as f32;
        writer.write_sample((sample * amplitude) as i16).unwrap();
    }
    // for t in (0 .. 44100).map(|x| x as f32 / 44100.0) {
    //     let sample = (t * 440.0 * 2.0 * PI).sin();
    //     let amplitude = i16::MAX as f32;
    //     writer.write_sample((sample * amplitude) as i16).unwrap();
    // }
}
