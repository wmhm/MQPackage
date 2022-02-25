// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::sync::{Arc, Mutex};

struct ProgressInternal<'p, T> {
    start: Option<Box<dyn FnMut(u64) -> T + 'p>>,
    update: Option<Box<dyn FnMut(&T, u64) + 'p>>,
    finish: Option<Box<dyn FnMut(&T) + 'p>>,
}

impl<'p, T> ProgressInternal<'p, T> {
    fn start(&mut self, len: u64) -> Option<T> {
        self.start.as_mut().map(|cb| (cb)(len))
    }

    fn update(&mut self, bar: &T, delta: u64) {
        if let Some(cb) = &mut self.update {
            (cb)(bar, delta);
        }
    }

    fn finish(&mut self, bar: &T) {
        if let Some(cb) = &mut self.finish {
            (cb)(bar);
        }
    }
}

pub(crate) struct Progress<'p, T> {
    internal: Arc<Mutex<ProgressInternal<'p, T>>>,
}

impl<'p, T> Progress<'p, T> {
    pub(crate) fn new() -> Progress<'p, T> {
        Progress {
            internal: Arc::new(Mutex::new(ProgressInternal {
                start: None,
                update: None,
                finish: None,
            })),
        }
    }

    pub(crate) fn with_progress_start(&mut self, cb: impl FnMut(u64) -> T + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.start = Some(Box::new(cb))
    }

    pub(crate) fn with_progress_update(&mut self, cb: impl FnMut(&T, u64) + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.update = Some(Box::new(cb))
    }

    pub(crate) fn with_progress_finish(&mut self, cb: impl FnMut(&T) + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.finish = Some(Box::new(cb))
    }
}

impl<'p, T> Progress<'p, T> {
    pub(crate) fn bar(&self, len: u64) -> ProgressBar<'p, T> {
        ProgressBar::new(self.internal.clone(), len)
    }
}

pub(crate) struct ProgressBar<'p, T> {
    bar: Option<Box<T>>,
    internal: Arc<Mutex<ProgressInternal<'p, T>>>,
}

impl<'p, T> ProgressBar<'p, T> {
    fn new(internal: Arc<Mutex<ProgressInternal<'p, T>>>, len: u64) -> ProgressBar<'p, T> {
        let mut lock = internal.lock().unwrap();
        let bar = lock.start(len).map(Box::new);

        drop(lock);

        ProgressBar { internal, bar }
    }

    pub(crate) fn update(&self, delta: u64) {
        if let Some(bar) = &self.bar {
            let mut internal = self.internal.lock().unwrap();
            internal.update(&**bar, delta);
        }
    }

    pub(crate) fn finish(&self) {
        if let Some(bar) = &self.bar {
            let mut internal = self.internal.lock().unwrap();
            internal.finish(&**bar);
        }
    }
}
