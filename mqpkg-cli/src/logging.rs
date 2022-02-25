// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use std::sync::{Arc, Mutex};

use indicatif::WeakProgressBar;
use log::{Metadata, Record};
use pretty_env_logger::env_logger::Logger;

struct IndicatifAwareLogger {
    internal: Logger,
    bars: Arc<Mutex<Vec<WeakProgressBar>>>,
}

impl IndicatifAwareLogger {
    fn new(internal: Logger, bars: Arc<Mutex<Vec<WeakProgressBar>>>) -> IndicatifAwareLogger {
        IndicatifAwareLogger { internal, bars }
    }

    fn install(self) {
        let max_level = self.internal.filter();

        log::set_boxed_logger(Box::new(self)).unwrap();
        log::set_max_level(max_level);
    }

    fn suspended(&self, callback: impl FnOnce()) {
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

impl log::Log for IndicatifAwareLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.internal.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.suspended(|| self.internal.log(record))
    }

    fn flush(&self) {}
}

pub(crate) fn setup(bars: Arc<Mutex<Vec<WeakProgressBar>>>) {
    let logger = IndicatifAwareLogger::new(
        pretty_env_logger::formatted_builder()
            .filter_level(log::LevelFilter::Trace)
            .build(),
        bars,
    );

    logger.install();
}
