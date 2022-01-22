use super::list::Skiplist;
use crate::common::{FixedLengthSuffixComparator, KeyComparator};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Memtable {
    list: Skiplist<FixedLengthSuffixComparator>,
    mem_next_logfile_number: AtomicU64,
}

impl Memtable {
    pub fn set_next_log_number(&self, num: u64) {
        self.mem_next_logfile_number.store(num, Ordering::Release);
    }

    pub fn get_next_log_number(&self) -> u64 {
        self.mem_next_logfile_number.load(Ordering::Acquire)
    }
}
