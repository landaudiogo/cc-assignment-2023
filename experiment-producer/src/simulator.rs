use rand::Rng;

#[derive(Clone, Copy)]
pub enum ExperimentStage {
    Configuration,
    Stabilization,
    CarryOut,
    Terminated,
}

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
    temp_range: TempRange,
}

impl TemperatureSample {
    pub fn is_out_of_range(&self) -> bool {
        self.cur > self.temp_range.upper_threshold || self.cur < self.temp_range.lower_threshold
    }

    pub fn cur(&self) -> f32 {
        self.cur
    }
}

pub struct Experiment {
    sample: TemperatureSample,
    stage: ExperimentStage,
}

impl Experiment {
    pub fn new(start: f32, temp_range: TempRange) -> Self {
        let sample = TemperatureSample {
            cur: start,
            temp_range,
        };
        Experiment {
            sample,
            stage: ExperimentStage::Configuration,
        }
    }

    pub fn iter_mut(&mut self, delta: f32, len: usize, random_range: f32) -> IterMut {
        IterMut {
            experiment: self,
            iteration: 0,
            delta,
            len,
            random_range,
        }
    }

    pub fn stabilization_samples(&mut self, len: usize) -> IterMut {
        let TempRange {
            lower_threshold,
            upper_threshold,
        } = self.sample.temp_range;
        let final_temperature = lower_threshold + (upper_threshold - lower_threshold) / 2_f32;
        let delta = (final_temperature - self.sample.cur) / (len as f32);
        self.iter_mut(delta, len, 0.0)
    }

    pub fn carry_out_samples(&mut self, len: usize) -> IterMut {
        let TempRange {
            lower_threshold,
            upper_threshold,
        } = self.sample.temp_range;
        self.iter_mut(0.0, len, upper_threshold - lower_threshold)
    }

    pub fn set_stage(&mut self, stage: ExperimentStage) {
        self.stage = stage;
    }

    pub fn stage(&self) -> ExperimentStage {
        self.stage
    }
}

pub struct IterMut<'a> {
    pub experiment: &'a mut Experiment,
    delta: f32,
    len: usize,
    iteration: usize,
    random_range: f32,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = TemperatureSample;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iteration >= self.len {
            return None;
        }
        let sample = &mut self.experiment.sample;

        sample.cur += self.delta;
        if self.random_range != 0.0 {
            let relative_val = rand::thread_rng().gen_range(-100.0..100.0);
            let absolute_val = relative_val * self.random_range / 100.0;
            sample.cur += absolute_val;
        }
        self.iteration += 1;
        Some(*sample)
    }
}

pub fn compute_sensor_temperatures(
    sensors: &Vec<String>,
    average_temperature: f32,
) -> Vec<(&'_ str, f32)> {
    let mut cumulative_temperature = 0.0;
    let mut sensor_events = sensors[..sensors.len() - 1]
        .into_iter()
        .map(|sensor_id| {
            let relative_diff = rand::thread_rng().gen_range(-100.0..100.0);
            let sensor_temperature = average_temperature + relative_diff * 1.0 / 100.0;
            cumulative_temperature += sensor_temperature;
            (&**sensor_id, sensor_temperature)
        })
        .collect::<Vec<(&'_ str, f32)>>();
    let last_sensor_id = &sensors[sensors.len() - 1];
    let last_sensor_temperature =
        (sensors.len() as f32) * average_temperature - cumulative_temperature;
    sensor_events.push((last_sensor_id, last_sensor_temperature));
    sensor_events
}
