use std::{iter, error};
use sample::signal;
use rand::{Rng, thread_rng};
use alsa::{seq, pcm};


use patchwork::source::*;
use patchwork::source::waves::*;
use patchwork::source::karplus_strong::*;
use patchwork::source::math::*;
use patchwork::alsa::{SAMPLE_RATE, open_audio_dev, open_midi_dev};

const TWOPI: f64 = std::f64::consts::PI * 2.0;

// Sample format
type SF = i16;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ADSRState {
    Attacking,
    Decaying,
    Sustaining,
    Releasing,
    Done
}

struct ADSR {
    attack_slope: f64,
    decay_slope: f64,
    sustain: f64,
    release_slope: f64,
    state: ADSRState,
    value: f64,
    source: Box<Rack>,
    note: u8,
}

impl ADSR {
    pub fn new(attack: f64, decay: f64, sustain: f64, release: f64, note: u8, source: Box<Rack>, sample_rate: u32) -> Self {
        assert!(0.0 <= sustain && sustain <= 1.0);

        let attack_steps = attack * sample_rate as f64;
        let decay_steps = decay * sample_rate as f64;
        let release_steps = release * sample_rate as f64;

        let attack_slope = 1.0 / attack_steps;
        let decay_slope = -((1.0 - sustain) / decay_steps);
        let release_slope = -(sustain / release_steps);

        Self {
            attack_slope,
            decay_slope,
            sustain,
            release_slope,
            state: ADSRState::Attacking,
            value: 0.0,
            source,
            note
        }
    }

    pub fn get(&mut self) -> f64 {
        let ret = self.value * self.source.get();

        match self.state {
            ADSRState::Done => (),
            ADSRState::Sustaining => (),
            ADSRState::Attacking => {
                self.value += self.attack_slope;
                if self.value >= 1.0 {
                    self.value = 1.0;
                    self.state = ADSRState::Decaying;
                }
            },
            ADSRState::Decaying => {
                self.value += self.decay_slope;
                if self.value <= self.sustain {
                    self.value = self.sustain;
                    self.state = ADSRState::Sustaining;
                }
            },
            ADSRState::Releasing => {
                self.value += self.release_slope;
                if self.value <= 0.0 {
                    self.value = 0.0;
                    self.state = ADSRState::Done;
                }
            },
        }

        ret
    }

    // pub fn note_on(&mut self) {
    //     self.state = ADSRState::Attacking;
    // }

    pub fn note_off(&mut self) {
        self.state = ADSRState::Releasing;
    }

    pub fn is_done(&self) -> bool {
        self.state == ADSRState::Done
    }
}

pub trait Module {
    fn get(&mut self) -> f64;
    fn set_input(&mut self, i: usize, val: f64);
}

#[derive(Debug, Clone)]
pub struct MPhase {
    value: f64,
    step: f64,
    sample_rate: u32,
}

impl MPhase {
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
pub struct MSine {
    phase: MPhase,
}

impl MSine {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: MPhase::new(freq, sample_rate) }
    }
}

impl Module for MSine {
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
pub struct MSquare0 {
    phase: MPhase,
}
impl MSquare0 {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: MPhase::new(freq, sample_rate) }
    }
}
impl Module for MSquare0 {
    fn get(&mut self) -> f64 {
        if self.phase.get() > 0.5 {
            1.0
        } else {
            0.0
        }
    }

    fn set_input(&mut self, i: usize, val: f64) {
        let freq = val * val * 880.0;
        // println!("Setting sq freq to {}", freq);
        match i {
            0 => self.phase.set_freq(freq),
            _ => ()
        }
    }
}

#[derive(Debug, Clone)]
pub struct MAvg {
    v0: f64,
    v1: f64,
    res: f64,
}
impl MAvg {
    pub fn new() -> Self {
        Self { v0: 0.0, v1: 0.0, res: 0.0 }
    }
}
impl Module for MAvg {
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
pub struct MScale {
    factor: f64,
    res: f64,
}
impl MScale {
    pub fn new(factor: f64) -> Self {
        Self { factor, res: 0.0 }
    }
}
impl Module for MScale {
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
pub struct MMult {
    v0: f64,
    v1: f64,
    res: f64,
}
impl MMult {
    pub fn new() -> Self {
        Self { v0: 0.0, v1: 0.0, res: 0.0 }
    }
}
impl Module for MMult {
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
pub struct MSaw {
    phase: MPhase,
}
impl MSaw {
    pub fn new(freq: f64, sample_rate: u32) -> Self {
        Self { phase: MPhase::new(freq, sample_rate) }
    }
}
impl Module for MSaw {
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

struct Rack {
    modules: Vec<Box<Module>>,
    buffer: Vec<f64>,
    buffer_back: Vec<f64>,
    output: Option<usize>,
    // output_id -> module_id, input_id
    patches: Vec<Vec<(usize, usize)>>,
    midi_inputs: usize,
}

/// A takes control events as inputs
/// and outputs a single `f64` signal.
///
/// It can contain multiple modules
/// with `f64` inputs and outputs
/// that can be connected through a shared bus.
impl Rack {
    pub fn new(midi_inputs: usize) -> Self {
        let mut buffer = Vec::new();
        let mut buffer_back = Vec::new();
        let mut patches = Vec::new();

        for _ in 0..midi_inputs {
            buffer.push(0.0);
            buffer_back.push(0.0);
            patches.push(Vec::new());
        }

        Self {
            modules: Vec::new(),
            buffer,
            buffer_back,
            patches,
            output: None,
            midi_inputs
        }
    }

    pub fn process_control(&mut self, param: u32, val: i32) {
        let param = param as usize;
        // Map value from 0..127 to 0.0...1.0
        let val = val as f64 / 127.0;

        // TODO: Change links to make this possible w/o buffer
        if param < self.midi_inputs {
            self.buffer[param] = val;
            self.buffer_back[param] = val;
            for (mod_id, input_id) in &self.patches[param] {
                self.modules[*mod_id - self.midi_inputs].set_input(*input_id, val);
            }
        }
    }

    /// Get the current output of the rack
    pub fn get(&mut self) -> f64 {
        let offset = self.midi_inputs;

        for i in 0..self.modules.len() {
            self.buffer_back[i + offset] = self.modules[i].get();
        }
        for i in 0..self.buffer.len() {
            let val = self.buffer_back[i];
            if val != self.buffer[i] {
                self.buffer[i] = val;
                for (mod_id, input_id) in &self.patches[i] {
                    self.modules[*mod_id - self.midi_inputs].set_input(*input_id, val);
                }
            }
        }

        if let Some(output) = self.output {
            self.buffer[output]
        } else {
            0.0
        }
    }

    pub fn fix_input(&mut self, i: usize, val: f64) {
        self.buffer[i] = val;
    }

    pub fn set_output(&mut self, i: usize) {
        self.output = Some(i);
    }

    pub fn register_module(&mut self, module: Box<Module>) -> usize {
        let id = self.modules.len();

        self.buffer.push(0.0);
        self.buffer_back.push(0.0);
        self.modules.push(module);
        self.patches.push(Vec::new());

        id + self.midi_inputs
    }

    // TODO: Prevent patching multiple outputs to one input
    pub fn patch(&mut self, output: usize, input: (usize, usize)) {
        self.patches[output].push(input);
    }
}

struct Synth {
    sigs: Vec<Option<ADSR>>,
    stored_sample: Option<SF>,
    writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
}

impl Synth {
    pub fn new(rate: u32) -> Self {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create("capture.wav", spec).unwrap();
        let mut sigs = Vec::new();
        for _ in 0..256 {
            sigs.push(None);
        }

        Self {
            sigs,
            stored_sample: None,
            writer,
        }
    }

    /// `note` ranges from 0 to 127
    /// `velocity` from 0 to 127,
    fn add_note(&mut self, note: u8, velocity: u8) {
        let hz = 440. * 2_f64.powf((note as f64 - 69.)/12.);

        let factor = (velocity as f64 / 128.0) / 10.0;

        let idx = self.sigs.iter().position(|s| s.is_none());
        let idx = if let Some(idx) = idx { idx } else {
            println!("Voice overflow!"); return;
        };
        // let sig = Avg::new(
        //     Box::new(Saw::new(hz, SAMPLE_RATE)),
        //     Box::new(Saw::new(hz + 2.0, SAMPLE_RATE)),
        // );
        // let sig = Mult::new(
        //     Box::new(sig),
        //     Box::new(Square0::new(hz * factor, SAMPLE_RATE)),
        // );
        // let sig = KarplusStrong::new(hz, SAMPLE_RATE);
        
        let mut rack = Rack::new(8);

        let saw_m = MSaw::new(hz, SAMPLE_RATE);
        let saw = rack.register_module(Box::new(saw_m));

        let saw2_m = MSine::new(hz * 2.0, SAMPLE_RATE);
        let saw2 = rack.register_module(Box::new(saw2_m));
        let scale_m = MScale::new(0.9);
        let scale = rack.register_module(Box::new(scale_m));
        rack.patch(saw2, (scale, 0));

        let avg_m = MMult::new();
        let avg = rack.register_module(Box::new(avg_m));
        rack.patch(saw, (avg, 0));
        rack.patch(scale, (avg, 1));

        let square_m = MSquare0::new(hz * 0.5 , SAMPLE_RATE);
        let square = rack.register_module(Box::new(square_m));
        // Control square freq with midi ctrl 1
        rack.patch(1, (square, 0));

        let mult_m = MMult::new();
        let mult = rack.register_module(Box::new(mult_m));

        // Combine square & saw signals
        rack.patch(avg, (mult, 0));
        rack.patch(square, (mult, 1));


        // for (i, patch) in rack.patches.iter().enumerate() {
        //     let strs: Vec<String> = patch.iter().map(|p| format!("{:?}", p)).collect();
        //     println!("{} -> {}", i, strs.join("; "));
        // }

        rack.set_output(mult);

        let envelope = ADSR::new(0.2, 0.1, 0.9, 1.0, note, Box::new(rack), SAMPLE_RATE);

        self.sigs[idx] = Some(envelope);
    }

    fn remove_note(&mut self, note: u8) {
        for i in self.sigs.iter_mut() {
            if let &mut Some(ref mut i) = i {
                if i.note == note { i.note_off() }
            }
        }
    }

    fn handle_control(&mut self, channel: u8, param: u32, val: i32) {
        for sig in self.sigs.iter_mut() {
            if let &mut Some(ref mut sig) = sig {
                sig.source.process_control(param, val);
            }
        }
    }
}

impl Iterator for Synth { 
    type Item = SF;
    fn next(&mut self) -> Option<Self::Item> {
        use sample::{Signal, Sample};

        // Mono -> Stereo
        if let Some(s) = self.stored_sample.take() { return Some(s) };
        
        let mut z = 0f64;
        for sig in &mut self.sigs { 
            let mut remove = false;
            if let &mut Some(ref mut i) = sig {
                z += i.get();
                if i.is_done() {
                    remove = true;
                }
            }
            if remove {
                *sig = None
            };
        }
        let z = z.min(0.999).max(-0.999);

        self.writer.write_sample(i16::from_sample(z)).unwrap();

        let z: Option<SF> = Some(SF::from_sample(z));
        self.stored_sample = z;
        z
    }
}


fn write_samples(p: &alsa::PCM, mmap: &mut alsa::direct::pcm::MmapPlayback<SF>, synth: &mut Synth)
    -> Result<bool, Box<error::Error>> {
    use alsa::pcm::State;

    // Write samples to DMA area from iterator
    if mmap.avail() > 0 {
        mmap.write(synth);
    }

    match mmap.status().state() {
        State::Running => { return Ok(false); }, // All fine
        State::Prepared => { println!("Starting audio output stream"); p.start()? },
        State::XRun => { println!("Underrun in audio output stream!"); p.prepare()? },
        State::Suspended => { println!("Resuming audio output stream"); p.resume()? },
        n @ _ => Err(format!("Unexpected pcm state {:?}", n))?,
    }
    Ok(true) // Call us again, please, there might be more data to write
}

fn read_midi_event(input: &mut seq::Input, synth: &mut Synth) -> Result<bool, Box<error::Error>> {
    if input.event_input_pending(true)? == 0 { return Ok(false); }
    let ev = input.event_input()?;
    // println!("Received: {:?}", ev);
    match ev.get_type() {
        seq::EventType::Noteon => {
            let data: seq::EvNote = ev.get_data().unwrap();
            if data.velocity == 0 {
                synth.remove_note(data.note);
            } else {
                synth.add_note(data.note, data.velocity);
            }
        },
        seq::EventType::Noteoff => {
            let data: seq::EvNote = ev.get_data().unwrap();
            synth.remove_note(data.note);
        },
        seq::EventType::Controller => {
            let data: seq::EvCtrl = ev.get_data().unwrap();
            synth.handle_control(data.channel, data.param, data.value);
        },
        _ => ()
    }
    Ok(true)
}


fn run() -> Result<(), Box<error::Error>> {
    let (audio_dev, rate) = open_audio_dev()?;
    let midi_dev = open_midi_dev()?;
    
    let mut midi_input = midi_dev.input();

    // 256 Voices synth
    let mut synth = Synth::new(rate);

    // Create an array of fds to poll.
    use alsa::PollDescriptors;
    let mut fds = audio_dev.get()?;
    fds.append(&mut (&midi_dev, Some(alsa::Direction::Capture)).get()?);
    
    // Let's use the fancy new "direct mode" for minimum overhead!
    let mut mmap = audio_dev.direct_mmap_playback::<SF>()?;
   
    loop {
        if write_samples(&audio_dev, &mut mmap, &mut synth)? { continue; }
        if read_midi_event(&mut midi_input, &mut synth)? { continue; }
        // Nothing to do, let's sleep until woken up by the kernel.
        alsa::poll::poll(&mut fds, 100)?;
    }
}

fn main() {
    if let Err(e) = run() {
        println!("Error ({}) {}", e.description(), e);
    }
}
