use crate::core::{MouseButton, RacError, RacResult};
use crate::thread::worker::ClickWorker;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::{Builder, JoinHandle};

pub struct ThreadManager {
    workers: HashMap<MouseButton, Arc<ClickWorker>>,
    handles: HashMap<MouseButton, Option<JoinHandle<()>>>,
}

impl Default for ThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadManager {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    pub fn register_worker(&mut self, worker: ClickWorker) {
        let button = worker.config().button;
        self.workers.insert(button, Arc::new(worker));
        self.handles.insert(button, None);
    }

    pub fn get_worker(&self, button: MouseButton) -> Option<Arc<ClickWorker>> {
        self.workers.get(&button).map(Arc::clone)
    }

    pub fn start_worker<F>(&mut self, button: MouseButton, worker_fn: F) -> RacResult<()>
    where
        F: FnOnce(Arc<ClickWorker>) + Send + 'static,
    {
        if self.handles.get(&button).and_then(|h| h.as_ref()).is_some() {
            self.stop_worker(button)?;
        }

        let worker = self.workers.get(&button).ok_or_else(|| {
            RacError::ThreadError(format!("Worker for {:?} button not registered", button))
        })?;

        let worker_clone = Arc::clone(worker);
        let worker_name = worker.config().name.clone();

        let handle = Builder::new()
            .name(worker_name.clone())
            .spawn(move || {
                worker_fn(worker_clone);
            })
            .map_err(|e| {
                RacError::ThreadError(format!("Failed to spawn thread {}: {}", worker_name, e))
            })?;

        self.handles.insert(button, Some(handle));
        Ok(())
    }

    pub fn stop_worker(&mut self, button: MouseButton) -> RacResult<()> {
        if let Some(worker) = self.workers.get(&button) {
            worker.signal().stop();
        }

        if let Some(handle_opt) = self.handles.get_mut(&button)
            && let Some(handle) = handle_opt.take()
        {
            handle.join().map_err(|e| {
                RacError::ThreadError(format!("Failed to join thread for {:?}: {:?}", button, e))
            })?;
        }

        Ok(())
    }

    pub fn stop_all(&mut self) -> RacResult<()> {
        let buttons: Vec<MouseButton> = self.workers.keys().copied().collect();

        for button in buttons {
            self.stop_worker(button)?;
        }

        Ok(())
    }

    pub fn start_signal(&self, button: MouseButton) -> RacResult<()> {
        let worker = self.workers.get(&button).ok_or_else(|| {
            RacError::ThreadError(format!("Worker for {:?} button not found", button))
        })?;

        worker.signal().start();
        Ok(())
    }

    pub fn pause_signal(&self, button: MouseButton) -> RacResult<()> {
        let worker = self.workers.get(&button).ok_or_else(|| {
            RacError::ThreadError(format!("Worker for {:?} button not found", button))
        })?;

        worker.signal().pause();
        Ok(())
    }
}

impl Drop for ThreadManager {
    fn drop(&mut self) {
        let _ = self.stop_all();
    }
}
