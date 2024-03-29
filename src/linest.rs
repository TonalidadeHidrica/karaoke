use druid::Data;
use std::ops::Range;

#[derive(Clone, Copy, Default, Debug, Data)]
pub struct Linest {
    x_sum: f64,
    x2_sum: f64,
    y_sum: f64,
    y2_sum: f64,
    xy_sum: f64,
    n: usize,
}

#[derive(Clone, Copy, Debug, Data)]
pub struct LinestResult {
    pub a: f64,
    pub b: f64,
    pub r2: f64,
}

impl Linest {
    pub fn push(&mut self, x: f64, y: f64) {
        self.x_sum += x;
        self.x2_sum += x * x;
        self.y_sum += y;
        self.y2_sum += y * y;
        self.xy_sum += x * y;
        self.n += 1;
    }

    pub fn estimate(&mut self) -> Option<LinestResult> {
        (self.n > 1).then(|| {
            let n = self.n as f64;
            let denom = n * self.x2_sum - self.x_sum * self.x_sum;
            let gue = n * self.xy_sum - self.x_sum * self.y_sum;
            let a = gue / denom;
            let b = (self.x2_sum * self.y_sum - self.xy_sum * self.x_sum) / denom;
            let r2 = gue * gue / denom / (n * self.y2_sum - self.y_sum * self.y_sum);
            LinestResult { a, b, r2 }
        })
    }
}

pub fn map_f64(x: f64, from: Range<f64>, to: Range<f64>) -> f64 {
    let from_size = from.end - from.start;
    let from_advance = x - from.start;
    assert!(from_size != 0.0 || from_advance == 0.0);
    let to_size = to.end - to.start;
    from_advance / from_size * to_size + to.start
}
