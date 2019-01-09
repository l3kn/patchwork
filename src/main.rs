use std::error;
use alsa::{seq};

// use patchwork::source::*;
// use patchwork::source::waves::*;
use patchwork::modules::*;
// use patchwork::source::karplus_strong::*;
// use patchwork::source::math::*;
use patchwork::util::{clamp, clamp_audio};
use patchwork::alsa::{open_audio_dev, open_midi_dev};
use patchwork::freeverb::Freeverb;
use patchwork::SAMPLE_RATE;

// Sample format
type SF = i16;

struct Rack {
    modules: Vec<Box<Module>>,
    buffer: Vec<f64>,
    buffer_back: Vec<f64>,
    output: Option<usize>,
    // output_id -> module_id, input_id
    patches: Vec<Vec<(usize, usize)>>,
    midi_inputs: usize,
    writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    freeverb: Freeverb,
    stored_sample: Option<i16>,
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

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE as u32,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let writer = hound::WavWriter::create("capture.wav", spec).unwrap();
        let mut freeverb = Freeverb::new();
        freeverb.set_room_size(0.4);

        Self {
            modules: Vec::new(),
            buffer,
            buffer_back,
            patches,
            output: None,
            midi_inputs,
            writer,
            freeverb,
            stored_sample: None,
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

impl Iterator for Rack { 
    type Item = SF;
    fn next(&mut self) -> Option<Self::Item> {
        use sample::Sample;

        // Mono -> Stereo
        if let Some(s) = self.stored_sample.take() { return Some(s) };
        
        let z = self.get().min(0.999).max(-0.999);
        let (l, r) = self.freeverb.process((z, z));

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

        self.writer.write_sample(i16::from_sample(l)).unwrap();
        self.writer.write_sample(i16::from_sample(r)).unwrap();

        let l: Option<SF> = Some(SF::from_sample(l));
        let r: Option<SF> = Some(SF::from_sample(r));
        self.stored_sample = r;
        l
    }
}


fn write_samples(p: &alsa::PCM, mmap: &mut alsa::direct::pcm::MmapPlayback<SF>, rack: &mut Rack)
    -> Result<bool, Box<error::Error>> {
    use alsa::pcm::State;

    // Write samples to DMA area from iterator
    if mmap.avail() > 0 {
        mmap.write(rack);
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

fn read_midi_event(input: &mut seq::Input, rack: &mut Rack) -> Result<bool, Box<error::Error>> {
    if input.event_input_pending(true)? == 0 { return Ok(false); }
    let ev = input.event_input()?;
    match ev.get_type() {
        seq::EventType::Controller => {
            let data: seq::EvCtrl = ev.get_data().unwrap();
            rack.process_control(data.param, data.value);
        },
        _ => ()
    }
    Ok(true)
}


fn run() -> Result<(), Box<error::Error>> {
    let (audio_dev, _rate) = open_audio_dev()?;
    let midi_dev = open_midi_dev()?;
    
    let mut midi_input = midi_dev.input();

    // 256 Voices synth
    let mut rack = Rack::new(8);

    let freq1 = rack.register_module(
        Box::new(LinMap::new(55.0, 220.0))
    );
    rack.patch(1, (freq1, 0));

    let freq2_f = rack.register_module(Box::new(LinMap::new(0.025, 1.0)));
    rack.patch(2, (freq2_f, 0));
    let freq3_f = rack.register_module(Box::new(LinMap::new(0.025, 1.0)));
    rack.patch(3, (freq3_f, 0));
    let freq4_f = rack.register_module(Box::new(LinMap::new(0.025, 1.0)));
    rack.patch(4, (freq4_f, 0));

    let freq2 = rack.register_module(Box::new(Mult::new()));
    rack.patch(freq1, (freq2, 0));
    rack.patch(freq2_f, (freq2, 1));

    let freq3 = rack.register_module(Box::new(Mult::new()));
    rack.patch(freq1, (freq3, 0));
    rack.patch(freq3_f, (freq3, 1));

    let freq4 = rack.register_module(Box::new(Mult::new()));
    rack.patch(freq1, (freq4, 0));
    rack.patch(freq4_f, (freq4, 1));

    type W = Triangle;
    let saw1_m = Triangle::new(220.0);
    let saw1 = rack.register_module(Box::new(saw1_m));
    rack.patch(freq1, (saw1, 0));

    let saw2_m = Saw::new(220.0);
    let saw2 = rack.register_module(Box::new(saw2_m));
    rack.patch(freq2, (saw2, 0));

    let saw3_m = Square::new(220.0);
    let saw3 = rack.register_module(Box::new(saw3_m));
    rack.patch(freq3, (saw3, 0));

    let saw4_m = Sine::new(220.0);
    let saw4 = rack.register_module(Box::new(saw4_m));
    rack.patch(freq4, (saw4, 0));

    let mix = rack.register_module(Box::new(
        Avg4::new()
    ));

    rack.patch(saw1, (mix, 0));
    rack.patch(saw2, (mix, 1));
    rack.patch(saw3, (mix, 2));
    rack.patch(saw4, (mix, 3));

    let vol = rack.register_module(Box::new(
        Mult::new()
    ));
    rack.patch(0, (vol, 0));
    rack.patch(mix, (vol, 1));
    rack.set_output(vol);

    // Create an array of fds to poll.
    use alsa::PollDescriptors;
    let mut fds = audio_dev.get()?;
    fds.append(&mut (&midi_dev, Some(alsa::Direction::Capture)).get()?);
    
    // Let's use the fancy new "direct mode" for minimum overhead!
    let mut mmap = audio_dev.direct_mmap_playback::<SF>()?;
   
    loop {
        if write_samples(&audio_dev, &mut mmap, &mut rack)? { continue; }
        if read_midi_event(&mut midi_input, &mut rack)? { continue; }
        // Nothing to do, let's sleep until woken up by the kernel.
        alsa::poll::poll(&mut fds, 100)?;
    }
}

fn main() {
    if let Err(e) = run() {
        println!("Error ({}) {}", e.description(), e);
    }
}
