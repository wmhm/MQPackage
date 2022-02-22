// This file is dual licensed under the terms of the Apache License, Version
// 2.0, and the BSD License. See the LICENSE file in the root of this repository
// for complete details.

use named_lock::{NamedLock, NamedLockGuard};
use thiserror::Error;

macro_rules! transaction {
    ($db:expr, $body:block) => {{
        let __txnm = $db.transaction()?;
        let __txn = $db.begin(&__txnm)?;
        let __result = $body;

        $db.commit(__txn)?;

        __result
    }};

    ($db:expr, $body:expr) => {{
        transaction!($db, { $body })
    }};
}

pub(crate) use transaction;

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error(transparent)]
    LockError(#[from] named_lock::Error),
}

#[derive(Debug)]
pub(crate) struct TransactionManager {
    lock: NamedLock,
}

#[derive(Debug)]
pub struct Transaction<'r> {
    _guard: NamedLockGuard<'r>,
}

impl TransactionManager {
    pub(super) fn new(id: &str) -> Result<TransactionManager, TransactionError> {
        Ok(TransactionManager {
            lock: NamedLock::create(&format!("mqpkg.{}", id))?,
        })
    }

    pub(super) fn begin(&self) -> Result<Transaction, TransactionError> {
        Ok(Transaction {
            _guard: self.lock.lock()?,
        })
    }
}
