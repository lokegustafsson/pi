#[derive(Clone, Copy)]
pub struct Show {
    value: f64,
    num_decimals: u8,
    scale: u8,
}
impl Show {
    const NUM: usize = 5;
    const RATES: [&str; Self::NUM] = ["B/s", "K/s", "M/s", "G/s", "T/s"];
    const SIZES: [&str; Self::NUM] = ["B", "KB", "MB", "GB", "TB"];
    const K: f64 = 1000.0;

    fn scale_of(num: f64) -> u8 {
        (num.max(1.1).log(Self::K) as usize).clamp(0, Self::NUM - 1) as u8
    }
    fn new(num: f64) -> Self {
        let scale = Self::scale_of(num);
        let value = num / Self::K.powi(scale as i32);
        Self {
            value,
            num_decimals: match () {
                _ if scale == 0 => 0,
                _ if value < 10.0 => 2,
                _ if value < 100.0 => 1,
                _ => 0,
            },
            scale,
        }
    }
    fn at_scale(num: f64, scale: u8) -> Self {
        let value = num / Self::K.powi(scale as i32);
        Self {
            value,
            num_decimals: match () {
                _ if scale == 0 => 0,
                _ if value < 10.0 => 2,
                _ if value < 100.0 => 1,
                _ => 0,
            },
            scale,
        }
    }

    pub fn size_at_scale(used: f64, reference: f64) -> String {
        let scale = Self::scale_of(reference);
        let Self {
            value,
            num_decimals,
            scale: _,
        } = Self::at_scale(used, scale);
        let unit = Self::SIZES[scale as usize];
        format!("{value:>4.*}{unit}", num_decimals as usize,)
    }
    pub fn size_fraction(used: f64, total: f64) -> String {
        let Self {
            value: tot_value,
            num_decimals: tot_decimals,
            scale,
        } = Self::new(total);
        let Self {
            value,
            num_decimals,
            scale: _,
        } = Self::at_scale(used, scale);
        let unit = Self::SIZES[scale as usize];
        format!(
            "{value:>4.*}/{tot_value:>4.*}{unit}",
            num_decimals as usize, tot_decimals as usize
        )
    }
    pub fn rate(rate: f64, prefix: &str) -> String {
        let Self {
            value,
            num_decimals,
            scale,
        } = Self::new(rate);
        let unit = Self::RATES[scale as usize];
        format!("{prefix}{value:>4.*}{unit}", num_decimals as usize)
    }
}
