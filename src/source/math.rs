use super::Source;
use crate::util::clamp;

pub struct Mult {
    sig1: Box<Source>,
    sig2: Box<Source>,
}

pub struct Avg {
    sig1: Box<Source>,
    sig2: Box<Source>,
}

pub struct Add {
    sig1: Box<Source>,
    sig2: Box<Source>,
}

impl Mult {
    pub fn new(sig1: Box<Source>, sig2: Box<Source>) -> Self {
        Self { sig1, sig2 }
    }
}

impl Avg {
    pub fn new(sig1: Box<Source>, sig2: Box<Source>) -> Self {
        Self { sig1, sig2 }
    }
}

impl Add {
    pub fn new(sig1: Box<Source>, sig2: Box<Source>) -> Self {
        Self { sig1, sig2 }
    }
}

impl Source for Mult {
    fn get(&mut self) -> f64 {
        self.sig1.get() * self.sig2.get()
    }

    fn copy(&self) -> Box<Source> {
        Box::new(Mult {
            sig1: self.sig1.copy(),
            sig2: self.sig2.copy(),
        })
    }
}

impl Source for Avg {
    fn get(&mut self) -> f64 {
        (self.sig1.get() + self.sig2.get()) * 0.5
    }

    fn copy(&self) -> Box<Source> {
        Box::new(Mult {
            sig1: self.sig1.copy(),
            sig2: self.sig2.copy(),
        })
    }
}

impl Source for Add {
    fn get(&mut self) -> f64 {
        clamp(self.sig1.get() + self.sig2.get(), -1.0, 1.0)
    }

    fn copy(&self) -> Box<Source> {
        Box::new(Mult {
            sig1: self.sig1.copy(),
            sig2: self.sig2.copy(),
        })
    }
}
