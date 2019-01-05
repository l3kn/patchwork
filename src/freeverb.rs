use crate::SAMPLE_RATE;

pub struct DelayLine {
    buffer: Vec<f64>,
    index: usize,
}

impl DelayLine {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length],
            index: 0,
        }
    }

    pub fn read(&self) -> f64 {
        self.buffer[self.index]
    }

    pub fn write(&mut self, value: f64) {
        self.buffer[self.index] = value;

        self.index += 1;
        if self.index >= self.buffer.len() {
            self.index = 0;
        }
    }
}

pub struct AllPass {
    delay_line: DelayLine,
}

impl AllPass {
    pub fn new(delay_length: usize) -> Self {
        let delay_length = convert_length(delay_length);
        Self { delay_line: DelayLine::new(delay_length) }
    }

    pub fn process(&mut self, input: f64) -> f64 {
        let delayed = self.delay_line.read();
        let output = -input + delayed;

        let feedback = 0.5;
        let next = input + delayed * feedback;

        self.delay_line.write(next);

        output
    }
}

pub struct Comb {
    delay_line: DelayLine,
    feedback: f64,
    filter_state: f64,
    dampening: f64,
    dampening_inv: f64,
}

impl Comb {
    pub fn new(delay_length: usize) -> Self {
        let delay_length = convert_length(delay_length);
        Self {
            delay_line: DelayLine::new(delay_length),
            feedback: 0.5,
            filter_state: 0.0,
            dampening: 0.5,
            dampening_inv: 0.5,
        }
    }

    pub fn set_dampening(&mut self, value: f64) {
        self.dampening = value;
        self.dampening_inv = 1.0 - value;
    }

    pub fn set_feedback(&mut self, value: f64) {
        self.feedback = value;
    }

    pub fn process(&mut self, input: f64) -> f64 {
        let output = self.delay_line.read();

        self.filter_state =
            output * self.dampening_inv +
            self.filter_state * self.dampening;

        let next = input + self.filter_state * self.feedback;
        self.delay_line.write(next);

        output
    }
}

const FIXED_GAIN: f64 = 0.015;
const SCALE_WET: f64 = 3.0;
const SCALE_DAMPENING: f64 = 0.4;
const SCALE_ROOM: f64 = 0.28;
const OFFSET_ROOM: f64 = 0.7;
const STEREO_SPREAD: usize = 23;
const COMB_TUNING: &[usize; 8] = &[1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: &[usize; 4] = &[225, 341, 441, 556];

pub struct Freeverb {
    combs: [(Comb, Comb); 8],
    allpasses: [(AllPass, AllPass); 4],
    wet_gains: (f64, f64),
    wet: f64,
    width: f64,
    dry: f64,
    dampening: f64,
    room_size: f64,
}

// TODO: Hardcode this into the values
fn convert_length(length: usize) -> usize {
    (length as f64 * SAMPLE_RATE / 44100.0) as usize
}

impl Freeverb {
    pub fn new() -> Self {
        let mut freeverb = Freeverb {
            combs: [
                (Comb::new(COMB_TUNING[0]), Comb::new(COMB_TUNING[0] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[1]), Comb::new(COMB_TUNING[1] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[2]), Comb::new(COMB_TUNING[2] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[3]), Comb::new(COMB_TUNING[3] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[4]), Comb::new(COMB_TUNING[4] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[5]), Comb::new(COMB_TUNING[5] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[6]), Comb::new(COMB_TUNING[6] + STEREO_SPREAD)),
                (Comb::new(COMB_TUNING[7]), Comb::new(COMB_TUNING[7] + STEREO_SPREAD)),
            ],
            allpasses: [
                (AllPass::new(ALLPASS_TUNING[0]), AllPass::new(ALLPASS_TUNING[0] + STEREO_SPREAD)),
                (AllPass::new(ALLPASS_TUNING[1]), AllPass::new(ALLPASS_TUNING[1] + STEREO_SPREAD)),
                (AllPass::new(ALLPASS_TUNING[2]), AllPass::new(ALLPASS_TUNING[2] + STEREO_SPREAD)),
                (AllPass::new(ALLPASS_TUNING[3]), AllPass::new(ALLPASS_TUNING[3] + STEREO_SPREAD)),
            ],
            wet_gains: (0.0, 0.0),
            wet: 0.0,
            dry: 0.0,
            width: 0.0,
            dampening: 0.0,
            room_size: 0.0,
        };

        freeverb.set_wet(1.0);
        freeverb.set_width(0.5);
        freeverb.set_dampening(0.5);
        freeverb.set_room_size(0.5);

        freeverb
    }

    pub fn process(&mut self, input: (f64, f64)) -> (f64, f64) {
        let input_mixed = (input.0 + input.1) * FIXED_GAIN;
        let mut out = (0.0, 0.0);

        for combs in self.combs.iter_mut() {
            out.0 += combs.0.process(input_mixed);
            out.1 += combs.1.process(input_mixed);
        }

        for allpasses in self.allpasses.iter_mut() {
            out.0 = allpasses.0.process(out.0);
            out.1 = allpasses.1.process(out.1);
        }

        (out.0 * self.wet_gains.0 + out.1 * self.wet_gains.1 + input.0 * self.dry,
         out.1 * self.wet_gains.0 + out.0 * self.wet_gains.1 + input.1 * self.dry)
    }

    pub fn set_dampening(&mut self, value: f64) {
        self.dampening = value * SCALE_DAMPENING;
        for combs in self.combs.iter_mut() {
            combs.0.set_dampening(self.dampening);
            combs.1.set_dampening(self.dampening);
        }
    }

    pub fn set_wet(&mut self, value: f64) {
        self.wet = value * SCALE_WET;
        self.update_wet_gains();
    }

    pub fn set_width(&mut self, value: f64) {
        self.width = value;
        self.update_wet_gains();
    }

    fn update_wet_gains(&mut self) {
        self.wet_gains = (
            self.wet * ((1.0 + self.width) / 2.0),
            self.wet * ((1.0 - self.width) / 2.0),
        );
    }

    pub fn set_room_size(&mut self, value: f64) {
        self.room_size = value * SCALE_ROOM + OFFSET_ROOM;
        for combs in self.combs.iter_mut() {
            combs.0.set_feedback(self.room_size);
            combs.1.set_feedback(self.room_size);
        }
    }

    pub fn set_dry(&mut self, value: f64) {
        self.dry = value;
    }
}
