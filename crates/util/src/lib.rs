use std::time::Duration;

pub const SUBSEC: u64 = 60;
pub const TICK_DELAY: Duration = Duration::from_micros(1_000_000 / SUBSEC);
pub const HISTORY: usize = (60 * SUBSEC + 1) as usize;

#[derive(Clone, Debug)]
pub struct WindowMovingAverage<const WINDOW_SIZE: usize> {
    i: usize,
    samples: [f64; WINDOW_SIZE],
}
impl<const WINDOW_SIZE: usize> WindowMovingAverage<WINDOW_SIZE> {
    #[must_use]
    pub fn smooth(&mut self, sample: f64) -> f64 {
        self.add(sample);
        self.get()
    }

    pub fn add(&mut self, sample: f64) {
        self.samples[self.i] = sample;
        self.i = (self.i + 1) % WINDOW_SIZE;
    }
    pub fn get(&self) -> f64 {
        self.samples.iter().copied().sum::<f64>() / WINDOW_SIZE as f64
    }
}
impl<const WINDOW_SIZE: usize> Default for WindowMovingAverage<WINDOW_SIZE> {
    fn default() -> Self {
        Self {
            i: 0,
            samples: [0.0; WINDOW_SIZE],
        }
    }
}

pub struct TextualKeyValue<'a> {
    pub key: &'static str,
    pub value: &'a mut u64,
}
impl<'a> TextualKeyValue<'a> {
    pub fn extract_from(schema: &mut [Option<Self>], data: &str) {
        let mut lines = data.lines();
        for TextualKeyValue { key, value } in schema.iter_mut().filter_map(|x| x.as_mut()) {
            loop {
                let line = lines.next().expect("key missing in data");
                if let Some(line) = line.strip_prefix(*key) {
                    **value = line
                        .strip_prefix(":")
                        .unwrap()
                        .trim_start_matches('\t')
                        .trim_start_matches(' ')
                        .split_ascii_whitespace()
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap();
                    break;
                }
            }
        }
    }
}
