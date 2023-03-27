use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct ResponseTime {
    histogram: Arc<Mutex<hdrhistogram::Histogram<u64>>>,
    clock: quanta::Clock,
    start: Option<u64>,
    total: Duration,
}

impl ResponseTime {
    pub fn new(clock: quanta::Clock) -> ResponseTime {
        Self {
            histogram: Arc::new(Mutex::new(hdrhistogram::Histogram::new(2).unwrap())),
            clock,
            start: None,
            total: Duration::default(),
        }
    }

    pub fn start(&mut self) {
        if self.start.is_none() {
            self.start = Some(self.clock.raw());
        }
    }

    pub fn pause(&mut self) {
        if let Some(t) = self.start {
            self.total += self.clock.delta(t, self.clock.raw());
            self.start = None;
        }
    }

    pub fn stop_and_record(&mut self) {
        self.pause();

        let val = max(self.total.as_micros(), 1);
        self.histogram.lock().unwrap().record(val as u64).unwrap();

        self.total = Duration::default();
    }

    pub fn is_empty(&self) -> bool {
        self.histogram.lock().unwrap().len() == 0
    }

    pub fn get_count(&self) -> u64 {
        self.histogram.lock().unwrap().len()
    }

    pub fn get_average(&self) -> Duration {
        Duration::from_micros(self.histogram.lock().unwrap().mean().round() as u64)
    }

    pub fn get_median(&self) -> Duration {
        Duration::from_micros(self.histogram.lock().unwrap().value_at_quantile(0.5))
    }

    pub fn get_p90(&self) -> Duration {
        Duration::from_micros(self.histogram.lock().unwrap().value_at_quantile(0.9))
    }

    pub fn get_p95(&self) -> Duration {
        Duration::from_micros(self.histogram.lock().unwrap().value_at_quantile(0.95))
    }
}

impl Clone for ResponseTime {
    fn clone(&self) -> Self {
        Self {
            histogram: self.histogram.clone(),
            clock: self.clock.clone(),
            start: None,
            total: Default::default(),
        }
    }
}
