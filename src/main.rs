use std::{iter, error};
use alsa::{seq, pcm};
use std::ffi::CString;
use sample::signal;
use rand::{Rng, thread_rng};

use patchwork::source::*;
use patchwork::source::waves::*;
use patchwork::source::math::*;

const TWOPI: f64 = std::f64::consts::PI * 2.0;
const SAMPLE_RATE: u32 = 48000;
const BUFFER_SIZE: i64 = 256;

fn connect_midi_source_ports(s: &alsa::Seq, our_port: i32) -> Result<(), Box<error::Error>> {
    // Iterate over clients and clients' ports
    let our_id = s.client_id()?;
    let ci = seq::ClientIter::new(&s);
    for client in ci {
        if client.get_client() == our_id { continue; } // Skip ourselves
        let pi = seq::PortIter::new(&s, client.get_client());
        for port in pi {
            let caps = port.get_capability();

            // Check that it's a normal input port
            if !caps.contains(seq::READ) || !caps.contains(seq::SUBS_READ) { continue; }
            if !port.get_type().contains(seq::MIDI_GENERIC) { continue; }

            // Connect source and dest ports
            let subs = seq::PortSubscribe::empty()?;
            subs.set_sender(seq::Addr { client: port.get_client(), port: port.get_port() });
            subs.set_dest(seq::Addr { client: our_id, port: our_port });
            println!("Reading from midi input {:?}", port);
            s.subscribe_port(&subs)?;
        }
    }

    Ok(())
} 

fn open_midi_dev() -> Result<alsa::Seq, Box<error::Error>> {
    // Open the sequencer.
    let s = alsa::Seq::open(None, Some(alsa::Direction::Capture), true)?;
    let cstr = CString::new("rust_synth_example").unwrap();
    s.set_client_name(&cstr)?;

    // Create a destination port we can read from
    let mut dinfo = seq::PortInfo::empty().unwrap();
    dinfo.set_capability(seq::WRITE | seq::SUBS_WRITE);
    dinfo.set_type(seq::MIDI_GENERIC | seq::APPLICATION);
    dinfo.set_name(&cstr);
    s.create_port(&dinfo).unwrap();
    let dport = dinfo.get_port();

    // source ports should ideally be configurable, but right now we're just reading them all.
    connect_midi_source_ports(&s, dport)?;

    Ok(s)
}

fn open_audio_dev() -> Result<(alsa::PCM, u32), Box<error::Error>> {
    // let args: Vec<_> = std::env::args().collect();
    // if args.len() < 2 { 
    //     println!("Usage: 'cargo run --release CARD_NAME SAMPLE_RATE BUF_SIZE'");
    //     Err("No card name specified")?
    // }
    // let req_devname = format!("hw:{}", args[1]);
    let req_devname = "hw:0";
    // Open the device
    let p = alsa::PCM::new(&req_devname, alsa::Direction::Playback, false)?;
    
    // Set hardware parameters
    {
        let hwp = pcm::HwParams::any(&p)?;
        hwp.set_channels(2)?;
        hwp.set_rate(SAMPLE_RATE, alsa::ValueOr::Nearest)?;
        hwp.set_format(pcm::Format::s16())?;
        hwp.set_access(pcm::Access::MMapInterleaved)?;
        hwp.set_buffer_size(BUFFER_SIZE)?;
        hwp.set_period_size(BUFFER_SIZE / 4, alsa::ValueOr::Nearest)?;
        p.hw_params(&hwp)?;
    }

    // Set software parameters
    let rate = {
        let hwp = p.hw_params_current()?;
        let swp = p.sw_params_current()?;
        let (bufsize, periodsize) = (hwp.get_buffer_size()?, hwp.get_period_size()?);
        swp.set_start_threshold(bufsize - periodsize)?;
        swp.set_avail_min(periodsize)?;
        p.sw_params(&swp)?;
        println!("Opened audio output {:?} with parameters: {:?}, {:?}", req_devname, hwp, swp);
        hwp.get_rate()?
    };

    Ok((p, rate))
}


// Sample format
type SF = i16;

struct Sig {
    note: u8,
    sig: Box<Source>,
    targetvol: f64,
    curvol: f64,
}

impl Clone for Sig {
    fn clone(&self) -> Sig {
        let copy = self.sig.copy();
        Sig {
            note: self.note,
            sig: copy,
            targetvol: self.targetvol,
            curvol: self.curvol
        }
    }
}

struct Synth {
    sigs: Vec<Option<Sig>>,
    sample_rate: signal::Rate,
    stored_sample: Option<SF>,
}

impl Synth {
    fn add_note(&mut self, note: u8, vol: f64) {
        let hz = 440. * 2_f64.powf((note as f64 - 69.)/12.);

        let idx = self.sigs.iter().position(|s| s.is_none());
        let idx = if let Some(idx) = idx { idx } else {
            println!("Voice overflow!"); return;
        };
        let sig = Mult::new(
            Box::new(Square::new(hz, SAMPLE_RATE)),
            Box::new(Saw::new(hz / 10.0, SAMPLE_RATE)),
        );
        // let sig = KarplusStrong::new(hz, SAMPLE_RATE);
        let sig = Saw::new(hz, SAMPLE_RATE);
        let s = Sig { sig: Box::new(sig), note, targetvol: vol, curvol: 0. };
        self.sigs[idx] = Some(s);
    }
    fn remove_note(&mut self, note: u8) {
        for i in self.sigs.iter_mut() {
            if let &mut Some(ref mut i) = i {
                if i.note == note { i.targetvol = 0. }
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
                let s = i.sig.get();
                z += s.mul_amp(i.curvol);

                // Quick and dirty volume envelope to avoid clicks. 
                if i.curvol != i.targetvol {
                    if i.targetvol == 0. {
                        i.curvol -= 0.002;
                        if i.curvol <= 0. { remove = true; }
                    } else {
                        i.curvol += 0.002;
                        if i.curvol >= i.targetvol { i.curvol = i.targetvol; }
                    }
                }
            }
            if remove { *sig = None };
        }
        let z = z.min(0.999).max(-0.999);
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
                synth.add_note(data.note, f64::from(data.velocity + 64) / 2048.);
            }
        },
        seq::EventType::Noteoff => {
            let data: seq::EvNote = ev.get_data().unwrap();
            synth.remove_note(data.note);
        },
        _ => {},
    }
    Ok(true)
}


fn run() -> Result<(), Box<error::Error>> {
    let (audio_dev, rate) = open_audio_dev()?;
    let midi_dev = open_midi_dev()?;
    
    let mut midi_input = midi_dev.input();

    // 256 Voices synth
    let mut synth = Synth {
        sigs: vec![None; 256],
        sample_rate: signal::rate(f64::from(rate)),
        stored_sample: None,
    };

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
    if let Err(e) = run() { println!("Error ({}) {}", e.description(), e); }
}
