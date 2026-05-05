use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rand::Rng;

use crate::executor;
use crate::types::{MacroStep, RunMode};

struct MacroEngineInner {
    cancel: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

/// Clone is cheap — shares worker/cancel state.
#[derive(Clone)]
pub struct MacroEngine {
    inner: Arc<MacroEngineInner>,
}

impl Default for MacroEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MacroEngineInner {
                cancel: Arc::new(AtomicBool::new(false)),
                running: Arc::new(AtomicBool::new(false)),
                worker: Mutex::new(None),
            }),
        }
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    pub fn interrupt(&self) {
        self.inner.cancel.store(true, Ordering::SeqCst);
    }

    pub fn spawn_run(&self, steps: Vec<MacroStep>, mode: RunMode, jitter_ms: Option<(u64, u64)>) {
        self.interrupt();
        thread::sleep(Duration::from_millis(2));
        self.inner.cancel.store(false, Ordering::SeqCst);
        self.inner.running.store(true, Ordering::SeqCst);

        let cancel = self.inner.cancel.clone();
        let running = self.inner.running.clone();
        let handle = thread::spawn(move || {
            match mode {
                RunMode::Once => run_sequence(&steps, &cancel, jitter_ms),
                RunMode::LoopUntilCancel => {
                    while !cancel.load(Ordering::SeqCst) {
                        run_sequence(&steps, &cancel, jitter_ms);
                    }
                }
            }
            running.store(false, Ordering::SeqCst);
        });

        if let Ok(mut g) = self.inner.worker.lock() {
            *g = Some(handle);
        }
    }
}

fn run_sequence(steps: &[MacroStep], cancel: &AtomicBool, jitter_ms: Option<(u64, u64)>) {
    let mut rng = rand::thread_rng();
    for step in steps {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        match step {
            MacroStep::Delay { ms } => {
                let mut total = *ms;
                if let Some((lo, hi)) = jitter_ms {
                    let hi = hi.max(lo);
                    total = total.saturating_add(rng.gen_range(lo..=hi));
                }
                sleep_interruptible(total, cancel);
            }
            other => {
                if executor::execute_step(other).is_err() {
                    break;
                }
            }
        }
    }
}

fn sleep_interruptible(total_ms: u64, cancel: &AtomicBool) {
    let mut left = total_ms;
    while left > 0 {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        let slice = left.min(10);
        thread::sleep(Duration::from_millis(slice));
        left = left.saturating_sub(slice);
    }
}
