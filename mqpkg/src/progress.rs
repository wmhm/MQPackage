// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::sync::{Arc, Mutex};

struct ProgressInternal<'p> {
    start: Option<Box<dyn FnMut(&str, u64) + 'p>>,
    update: Option<Box<dyn FnMut(&str, u64) + 'p>>,
    finish: Option<Box<dyn FnMut(&str) + 'p>>,
}

impl<'p> ProgressInternal<'p> {
    fn start(&mut self, id: &str, len: u64) {
        if let Some(cb) = &mut self.start {
            (cb)(id, len);
        }
    }

    fn update(&mut self, id: &str, delta: u64) {
        if let Some(cb) = &mut self.update {
            (cb)(id, delta);
        }
    }

    fn finish(&mut self, id: &str) {
        if let Some(cb) = &mut self.finish {
            (cb)(id);
        }
    }
}

pub(crate) struct Progress<'p> {
    internal: Arc<Mutex<ProgressInternal<'p>>>,
}

impl<'p> Progress<'p> {
    pub(crate) fn new() -> Progress<'p> {
        Progress {
            internal: Arc::new(Mutex::new(ProgressInternal {
                start: None,
                update: None,
                finish: None,
            })),
        }
    }

    pub(crate) fn with_progress_start(&mut self, cb: impl FnMut(&str, u64) + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.start = Some(Box::new(cb))
    }

    pub(crate) fn with_progress_update(&mut self, cb: impl FnMut(&str, u64) + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.update = Some(Box::new(cb))
    }

    pub(crate) fn with_progress_finish(&mut self, cb: impl FnMut(&str) + 'p) {
        let mut internal = self.internal.lock().unwrap();
        internal.finish = Some(Box::new(cb))
    }
}

impl<'p> Progress<'p> {
    pub(crate) fn bar(&self, name: &str, len: u64) -> ProgressBar<'p> {
        ProgressBar::new(self.internal.clone(), name, len)
    }
}

pub(crate) struct ProgressBar<'p> {
    name: String,
    internal: Arc<Mutex<ProgressInternal<'p>>>,
}

impl<'p> ProgressBar<'p> {
    fn new<S: Into<String>>(
        internal: Arc<Mutex<ProgressInternal<'p>>>,
        name: S,
        len: u64,
    ) -> ProgressBar<'p> {
        let name = name.into();
        let bar = ProgressBar { name, internal };

        bar.start(len);
        bar
    }

    fn start(&self, len: u64) {
        let mut internal = self.internal.lock().unwrap();
        internal.start(self.name.as_str(), len);
    }

    pub(crate) fn update(&self, delta: u64) {
        let mut internal = self.internal.lock().unwrap();
        internal.update(self.name.as_str(), delta);
    }

    pub(crate) fn finish(&self) {
        let mut internal = self.internal.lock().unwrap();
        internal.finish(self.name.as_str());
    }
}
