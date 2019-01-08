pub fn clamp(i: f64, min: f64, max: f64) -> f64 {
    if i < min {
        min
    } else if i > max {
        max
    } else {
        i
    }
}

pub fn clamp_audio(i: f64) -> f64 {
    if i < -1.0 {
        -1.0
    } else if i > 1.0 {
        1.0
    } else {
        i
    }
}
