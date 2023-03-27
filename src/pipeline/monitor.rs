use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use console::{style, Color};
use flume::{Receiver, Sender};
use human_repr::HumanDuration;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressFinish, ProgressStyle};
use quanta::{Clock, Instant};

use crate::metrics::ResponseTime;

pub struct PipelineMonitor {
    clock: Clock,
    threads: Mutex<Vec<thread::JoinHandle<()>>>,
    pg_multi: indicatif::MultiProgress,
}

impl PipelineMonitor {
    pub fn new() -> PipelineMonitor {
        Self {
            clock: Clock::new(),
            threads: Mutex::new(Vec::new()),
            pg_multi: indicatif::MultiProgress::new(),
        }
    }

    pub fn create_step<'a, I, O>(
        &'a self,
        name: &'static str,
        rx: &'a Receiver<I>,
        tx: &'a Sender<O>,
    ) -> PipelineStep<I, O>
    where
        I: Send + Sync,
        O: Send + Sync,
    {
        let pg_bar = self.pg_multi.add(
            ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr()).with_finish(ProgressFinish::Abandon),
        );

        let mut pg_style = String::from("{spinner} ");
        pg_style.push_str(format!("{:18}", name).as_str());
        pg_style.push_str("  {prefix:.dim}  {wide_msg}");
        pg_bar.set_style(ProgressStyle::with_template(pg_style.as_str()).unwrap());

        PipelineStep::new(self.clock.clone(), self, name, rx, tx, pg_bar)
    }

    fn add_thread(&self, handle: thread::JoinHandle<()>) {
        self.threads.lock().unwrap().push(handle);
    }

    pub fn join_threads(&self) {
        for t in self.threads.lock().unwrap().drain(..) {
            let _ = t.join();
        }
        self.pg_multi.clear().unwrap();
    }
}

pub struct PipelineStep<'a, I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    monitor: &'a PipelineMonitor,
    name: &'static str,
    threads: u16,
    rx: &'a Receiver<I>,
    tx: &'a Sender<O>,
    progress: PipelineStepProgress,
}

impl<'a, I, O> PipelineStep<'a, I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn new(
        clock: Clock,
        monitor: &'a PipelineMonitor,
        name: &'static str,
        rx: &'a Receiver<I>,
        tx: &'a Sender<O>,
        pg_bar: ProgressBar,
    ) -> Self {
        Self {
            monitor,
            name,
            threads: 0,
            rx,
            tx,
            progress: PipelineStepProgress::new(name, clock, pg_bar),
        }
    }

    pub fn spawn_thread<F, T>(&mut self, f: F)
    where
        F: Fn(PipelineThreadContext<I, O>) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.threads += 1;
        self.progress.set_threads(self.threads);

        let ctx = PipelineThreadContext::new(
            self.name,
            self.threads,
            self.rx.clone(),
            self.tx.clone(),
            self.progress.clone(),
        );

        let handle = thread::Builder::new()
            .name(ctx.get_thread_name())
            .spawn(move || {
                f(ctx);
            })
            .unwrap();

        self.monitor.add_thread(handle);
    }
}

pub struct PipelineThreadContext<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    step_name: &'static str,
    thread_num: u16,
    rx: Receiver<I>,
    tx: Sender<O>,
    progress: PipelineStepProgress,
}

impl<I, O> PipelineThreadContext<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    fn new(
        step_name: &'static str,
        thread_num: u16,
        rx: Receiver<I>,
        tx: Sender<O>,
        progress: PipelineStepProgress,
    ) -> Self {
        Self {
            step_name,
            thread_num,
            rx,
            tx,
            progress,
        }
    }

    fn get_thread_name(&self) -> String {
        format!("{} {}", self.step_name, self.thread_num)
    }

    pub fn println(&self, msg: &str) {
        self.progress.pg_bar.println(msg);
    }

    pub fn recv(&mut self) -> Result<I> {
        self.progress.on_before_recv();

        let result = self
            .rx
            .recv()
            .with_context(|| format!("{} {}: receive channel closed", self.step_name, self.thread_num))?;

        self.progress.on_after_recv();

        Ok(result)
    }

    pub fn send(&mut self, msg: O) {
        self.progress.on_before_send();

        self.tx
            .send(msg)
            .with_context(|| format!("{} {}: send channel closed", self.step_name, self.thread_num))
            .unwrap();

        self.progress.on_after_send();
    }
}

const UPDATE_PROGRESS_FREQUENCY: Duration = Duration::from_millis(300);

struct PipelineStepProgress {
    step_name: &'static str,
    clock: Clock,
    last_update: Arc<Mutex<Instant>>,
    pg_bar: ProgressBar,
    instances: Arc<AtomicU32>,
    recv_time: ResponseTime,
    process_time: ResponseTime,
    send_time: ResponseTime,
}

impl PipelineStepProgress {
    fn new(step_name: &'static str, clock: Clock, pg_bar: ProgressBar) -> Self {
        Self {
            step_name,
            clock: clock.clone(),
            last_update: Arc::new(Mutex::new(clock.now() - UPDATE_PROGRESS_FREQUENCY)),
            pg_bar,
            instances: Arc::new(AtomicU32::new(1)),
            recv_time: ResponseTime::new(clock.clone()),
            process_time: ResponseTime::new(clock.clone()),
            send_time: ResponseTime::new(clock),
        }
    }

    fn set_threads(&self, threads: u16) {
        self.pg_bar.set_prefix(format!("[{:2} threads]", threads));
    }

    fn on_before_recv(&mut self) {
        self.process_time.pause();
        self.recv_time.start();
    }

    fn on_after_recv(&mut self) {
        self.recv_time.stop_and_record();
        self.update_progress();
        self.process_time.start();
    }

    fn on_before_send(&mut self) {
        self.process_time.stop_and_record();
        self.update_progress();
        self.send_time.start();
    }

    fn on_after_send(&mut self) {
        self.send_time.stop_and_record();
        self.update_progress();
        self.process_time.start();
    }

    fn update_progress(&self) {
        if self.recv_time.is_empty() {
            return;
        }

        if let Ok(mut lu) = self.last_update.try_lock() {
            let now = self.clock.now();

            if now - *lu > UPDATE_PROGRESS_FREQUENCY {
                self.force_update_progress();
                *lu = now;
            }
        }
    }

    fn force_update_progress(&self) {
        let s = self.clock.raw();

        if self.recv_time.is_empty() {
            return;
        }

        let total =
            (self.recv_time.get_average() + self.process_time.get_average() + self.send_time.get_average()).as_micros();

        let to_str = |name, time: &ResponseTime| {
            let avg = time.get_average();

            let result = format!(
                "{}: {:5} avg: {:7} p95: {:7}",
                name,
                time.get_count(),
                avg.human_duration().to_string(),
                time.get_p95().human_duration().to_string()
            );

            let avg = avg.as_micros();
            if avg < total / 10 {
                style(result).fg(Color::Green).to_string()
            } else if avg > total * 8 / 10 {
                style(result).fg(Color::Red).to_string()
            } else if avg > total * 4 / 10 {
                style(result).fg(Color::Yellow).to_string()
            } else {
                result
            }
        };

        let mut msg = String::new();
        msg.push_str(to_str("RX", &self.recv_time).as_str());

        if !self.process_time.is_empty() {
            msg.push_str(" >> ");
            msg.push_str(to_str("Run", &self.process_time).as_str());
        }

        if !self.send_time.is_empty() {
            msg.push_str(" >> ");
            msg.push_str(to_str("TX", &self.send_time).as_str());
        }

        self.pg_bar.set_message(msg);
        self.pg_bar.tick();
    }
}

impl Clone for PipelineStepProgress {
    fn clone(&self) -> Self {
        self.instances.fetch_add(1, Ordering::SeqCst);

        Self {
            step_name: self.step_name,
            clock: self.clock.clone(),
            last_update: self.last_update.clone(),
            pg_bar: self.pg_bar.clone(),
            instances: self.instances.clone(),
            recv_time: self.recv_time.clone(),
            process_time: self.process_time.clone(),
            send_time: self.send_time.clone(),
        }
    }
}

impl Drop for PipelineStepProgress {
    fn drop(&mut self) {
        let i = self.instances.fetch_sub(1, Ordering::SeqCst);

        self.force_update_progress();

        if i == 1 {
            self.pg_bar.set_prefix("[ finished ]");
        }
    }
}
