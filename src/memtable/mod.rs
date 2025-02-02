mod arena;
mod list;
mod memtable;

pub use memtable::Memtable;

const MAX_HEIGHT: usize = 20;

pub use list::Skiplist;
