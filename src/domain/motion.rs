//! Converts continuous movement into pixels without losing sub-pixel motion.
#[derive(Default)]
pub struct PixelAccumulator {
    x: f64,
    y: f64,
}

impl PixelAccumulator {
    pub fn add(&mut self, x: f64, y: f64) -> Option<(i32, i32)> {
        self.x += x;
        self.y += y;
        let pixels = (self.x.trunc() as i32, self.y.trunc() as i32);
        self.x -= f64::from(pixels.0);
        self.y -= f64::from(pixels.1);
        (pixels != (0, 0)).then_some(pixels)
    }

    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_movements_accumulate_and_reverse_without_bias() {
        let mut pixels = PixelAccumulator::default();
        assert_eq!(pixels.add(0.4, -0.4), None);
        assert_eq!(pixels.add(0.4, -0.4), None);
        assert_eq!(pixels.add(0.4, -0.4), Some((1, -1)));
        assert_eq!(pixels.add(-0.2, 0.2), None);
        pixels.reset();
        assert_eq!(pixels.add(0.9, 0.0), None);
    }
}
