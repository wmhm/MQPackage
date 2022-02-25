// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use indicatif::{ProgressBar, WeakProgressBar};
use std::sync::{Arc, Mutex};

pub(crate) struct SuspendableBars {
    bars: Arc<Mutex<Vec<WeakProgressBar>>>,
}

impl Clone for SuspendableBars {
    fn clone(&self) -> SuspendableBars {
        SuspendableBars {
            bars: self.bars.clone(),
        }
    }
}

impl SuspendableBars {
    pub(crate) fn new() -> SuspendableBars {
        SuspendableBars {
            bars: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn with_bar(&self, bar: ProgressBar) -> ProgressBar {
        self.bars.lock().unwrap().push(bar.downgrade());
        bar
    }

    pub(crate) fn suspended(&self, callback: impl FnOnce()) {
        let mut bs = self.bars.lock().unwrap();
        bs.retain(|b| b.upgrade().is_some());

        let mut ibar = bs.iter().filter(|bar| match bar.upgrade() {
            Some(b) => !b.is_finished(),
            None => false,
        });
        let wbar = ibar.next();

        match wbar {
            Some(wbar) => {
                let nbars = ibar.count();
                assert!(nbars == 0, "too many active bars");

                match wbar.upgrade() {
                    Some(bar) => bar.suspend(callback),
                    None => (callback)(),
                }
            }
            None => (callback)(),
        }
    }
}
