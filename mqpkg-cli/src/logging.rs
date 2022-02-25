// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use log::{LevelFilter, Metadata, Record};
use pretty_env_logger::env_logger::Logger;

use crate::progress::SuspendableBars;

struct IndicatifAwareLogger {
    internal: Logger,
    bars: SuspendableBars,
}

impl IndicatifAwareLogger {
    fn new(internal: Logger, bars: SuspendableBars) -> IndicatifAwareLogger {
        IndicatifAwareLogger { internal, bars }
    }

    fn install(self) {
        let max_level = self.internal.filter();

        log::set_boxed_logger(Box::new(self)).unwrap();
        log::set_max_level(max_level);
    }
}

impl log::Log for IndicatifAwareLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.internal.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        self.bars.suspended(|| self.internal.log(record))
    }

    fn flush(&self) {}
}

pub(crate) fn setup(level: LevelFilter, bars: SuspendableBars) {
    let logger = IndicatifAwareLogger::new(
        pretty_env_logger::formatted_builder()
            .filter_level(level)
            .build(),
        bars,
    );

    logger.install();
}
