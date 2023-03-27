use std::sync::Mutex;
use std::thread;

use anyhow::{Context, Result};
use flume::{Receiver, Sender};

pub struct PipelineMonitor {
    threads: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl PipelineMonitor {
    pub fn new() -> PipelineMonitor {
        Self {
            threads: Mutex::new(Vec::new()),
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
        PipelineStep {
            monitor: &self,
            name,
            threads: 0,
            rx,
            tx,
        }
    }

    fn add_thread(&self, handle: thread::JoinHandle<()>) {
        self.threads.lock().unwrap().push(handle);
    }

    pub fn join_threads(&self) {
        for t in self.threads.lock().unwrap().drain(..) {
            let _ = t.join();
        }
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
}

impl<'a, I, O> PipelineStep<'a, I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn spawn_thread<F, T>(&mut self, f: F)
    where
        F: Fn(PipelineThreadContext<I, O>) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.threads += 1;

        let ctx = PipelineThreadContext {
            step_name: self.name,
            thread_num: self.threads,
            rx: self.rx.clone(),
            tx: self.tx.clone(),
        };

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
}

impl<I, O> PipelineThreadContext<I, O>
where
    I: Send + Sync + 'static,
    O: Send + Sync + 'static,
{
    pub fn get_thread_name(&self) -> String {
        format!("{} {}", self.step_name, self.thread_num)
    }

    pub fn recv(&self) -> Result<I> {
        let result = self
            .rx
            .recv()
            .with_context(|| format!("{} {}: receive channel closed", self.step_name, self.thread_num))?;

        Ok(result)
    }

    pub fn send(&self, msg: O) {
        self.tx
            .send(msg)
            .with_context(|| format!("{} {}: send channel closed", self.step_name, self.thread_num))
            .unwrap();
    }
}
