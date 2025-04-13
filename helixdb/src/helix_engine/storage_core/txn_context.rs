use heed3::{RoTxn, RwTxn};
use std::fmt;

pub struct TransactionContext<'env> {
    read_txn: Option<RoTxn<'env>>,
    write_txn: Option<RwTxn<'env>>,
}

impl<'env> TransactionContext<'env> {
    #[inline]
    pub fn new() -> Self {
        Self {
            read_txn: None,
            write_txn: None,
        }
    }

    #[inline]
    pub fn with_read_txn(txn: RoTxn<'env>) -> Self {
        Self {
            read_txn: Some(txn),
            write_txn: None,
        }
    }

    #[inline]
    pub fn with_write_txn(txn: RwTxn<'env>) -> Self {
        Self {
            read_txn: None,
            write_txn: Some(txn),
        }
    }

    #[inline(always)]
    pub fn has_read_txn(&self) -> bool {
        self.read_txn.is_some()
    }

    #[inline(always)]
    pub fn has_write_txn(&self) -> bool {
        self.write_txn.is_some()
    }

    #[inline(always)]
    pub fn get_read_txn(&self) -> Option<&RoTxn<'env>> {
        self.read_txn.as_ref()
    }

    #[inline(always)]
    pub fn get_write_txn(&mut self) -> Option<&mut RwTxn<'env>> {
        self.write_txn.as_mut()
    }

    #[inline(always)]
    pub fn take_read_txn(&mut self) -> Option<RoTxn<'env>> {
        self.read_txn.take()
    }

    #[inline(always)]
    pub fn take_write_txn(&mut self) -> Option<RwTxn<'env>> {
        self.write_txn.take()
    }

    #[inline(always)]
    pub fn set_read_txn(&mut self, txn: RoTxn<'env>) {
        self.read_txn = Some(txn);
    }

    #[inline(always)]
    pub fn set_write_txn(&mut self, txn: RwTxn<'env>) {
        self.write_txn = Some(txn);
    }

    #[inline(always)]
    pub fn commit(&mut self) -> Result<(), heed3::Error> {
        if let Some(txn) = self.write_txn.take() {
            txn.commit()?;
        }
        Ok(())
    }

    // Abort write transaction if it exists
    #[inline(always)]
    pub fn abort(&mut self) {
        if let Some(txn) = self.write_txn.take() {
            drop(txn);
        }
    }
}

impl<'env> Default for TransactionContext<'env> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'env> Drop for TransactionContext<'env> {
    fn drop(&mut self) {
        if let Some(txn) = self.write_txn.take() {
            drop(txn);
        }
    }
}

impl<'env> fmt::Display for TransactionContext<'env> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TransactionContext {{ read_txn: {}, write_txn: {} }}",
            self.has_read_txn(),
            self.has_write_txn()
        )
    }
}

impl<'env> fmt::Debug for TransactionContext<'env> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TransactionContext {{ read_txn: {}, write_txn: {} }}",
            self.has_read_txn(),
            self.has_write_txn()
        )
    }
}
