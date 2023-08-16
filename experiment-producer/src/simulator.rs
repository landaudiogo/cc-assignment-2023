use rand::Rng;


#[derive(Clone, Copy, Debug)]
pub struct TempRange {
    lower_threshold: f32,
    upper_threshold: f32,
}

impl TempRange {
    pub fn new(lower_threshold: f32, upper_threshold: f32) -> Option<Self> {
        if lower_threshold > upper_threshold {
            return None;
        }
        Some(Self {
            upper_threshold,
            lower_threshold,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TemperatureSample {
    cur: f32,
    temp_range: TempRange
}

impl TemperatureSample {
    pub fn into_iter(self, delta: f32, len: usize, random_range: f32) -> IntoIter {
        IntoIter {
            sample: self,
            iteration: 0,
            delta,
            len,
            random_range,
        }
    }

    pub fn is_in_range(&self) -> bool {
        self.cur > self.temp_range.lower_threshold
        && self.cur < self.temp_range.upper_threshold
    }

    pub fn cur(&self) -> f32 {
        self.cur
    }
}

pub struct IntoIter {
    sample: TemperatureSample,
    delta: f32,
    len: usize,
    iteration: usize,
    random_range: f32,
}

impl Iterator for IntoIter {
    type Item = TemperatureSample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iteration >= self.len {
            return None;
        }

        self.sample.cur += self.delta;
        if self.random_range != 0.0 {
            let relative_val = rand::thread_rng().gen_range(-100.0..100.0);
            let absolute_val = relative_val * self.random_range / 100.0;
            self.sample.cur += absolute_val;
        }
        self.iteration += 1;
        Some(self.sample)
    }
}

pub fn stabilization_samples(start: f32, temperature_range: TempRange, len: usize) -> IntoIter {
    let TempRange {
        lower_threshold,
        upper_threshold,
    } = temperature_range;
    let final_temperature = lower_threshold + (upper_threshold - lower_threshold) / 2_f32;
    let delta = (final_temperature - start) / (len as f32);
    TemperatureSample { cur: start, temp_range: temperature_range }.into_iter(delta, len, 0.0)
}

fn carry_out_samples(sample: TemperatureSample, len: usize) { }
