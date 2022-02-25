// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::fmt;
use std::sync::{Arc, Mutex};

struct ProgressInternal<'p, T> {
    spinner: Option<Box<dyn FnMut(&'static str) -> T + 'p>>,
    bar: Option<Box<dyn FnMut(u64) -> T + 'p>>,
    update: Option<Box<dyn FnMut(&T, u64) + 'p>>,
    finish: Option<Box<dyn FnMut(&T) + 'p>>,
}

impl<'p, T> fmt::Debug for ProgressInternal<'p, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressInternal").finish()
    }
}

impl<'p, T> ProgressInternal<'p, T> {
    fn bar(&mut self, len: u64) -> Option<T> {
        self.bar.as_mut().map(|cb| (cb)(len))
    }

    fn spinner(&mut self, msg: &'static str) -> Option<T> {
        self.spinner.as_mut().map(|cb| (cb)(msg))
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

#[derive(Debug)]
pub(crate) struct Progress<'p, T> {
    internal: Arc<Mutex<ProgressInternal<'p, T>>>,
}

impl<'p, T> Clone for Progress<'p, T> {
    fn clone(&self) -> Self {
        Progress {
            internal: self.internal.clone(),
        }
    }
}

impl<'p, T> Progress<'p, T> {
    pub(crate) fn new() -> Progress<'p, T> {
        Progress {
            internal: Arc::new(Mutex::new(ProgressInternal {
                bar: None,
                update: None,
                finish: None,
                spinner: None,
            })),
        }
    }

    pub(crate) fn with_progress_start(&mut self, cb: impl FnMut(u64) -> T + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.bar = Some(Box::new(cb))
    }

    pub(crate) fn with_progress_spinner(&mut self, cb: impl FnMut(&'static str) -> T + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.spinner = Some(Box::new(cb))
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

    pub(crate) fn spinner(&self, msg: &'static str) -> ProgressBar<'p, T> {
        ProgressBar::new_spinner(self.internal.clone(), msg)
    }
}

pub(crate) struct ProgressBar<'p, T> {
    bar: Option<Box<T>>,
    internal: Arc<Mutex<ProgressInternal<'p, T>>>,
}

impl<'p, T> ProgressBar<'p, T> {
    fn new(internal: Arc<Mutex<ProgressInternal<'p, T>>>, len: u64) -> ProgressBar<'p, T> {
        let mut lock = internal.lock().unwrap();
        let bar = lock.bar(len).map(Box::new);

        drop(lock);

        ProgressBar { internal, bar }
    }

    fn new_spinner(
        internal: Arc<Mutex<ProgressInternal<'p, T>>>,
        msg: &'static str,
    ) -> ProgressBar<'p, T> {
        let mut lock = internal.lock().unwrap();
        let bar = lock.spinner(msg).map(Box::new);

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
