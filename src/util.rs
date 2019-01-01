pub fn clamp(i: f64, min: f64, max: f64) -> f64 {
    if i < min {
        min
    } else if i > max {
        max
    } else {
        i
    }
}
