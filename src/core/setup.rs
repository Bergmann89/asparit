use std::cmp::{max, min};

pub trait WithSetup {
    /// Setup to drive the iterator with.
    fn setup(&self) -> Setup {
        Setup::default()
    }
}

#[derive(Debug, Default, Clone)]
pub struct Setup {
    /// Number of splits/threads this iterator will use to proceed.
    pub splits: Option<usize>,

    /// The minimum number of items that we will process
    /// sequentially. Defaults to 1, which means that we will split
    /// all the way down to a single item. This can be raised higher
    /// using the [`with_min_len`] method, which will force us to
    /// create sequential tasks at a larger granularity. Note that
    /// Rayon automatically normally attempts to adjust the size of
    /// parallel splits to reduce overhead, so this should not be
    /// needed.
    ///
    /// [`with_min_len`]: ../trait.IndexedParallelIterator.html#method.with_min_len
    pub min_len: Option<usize>,

    /// The maximum number of items that we will process
    /// sequentially. Defaults to MAX, which means that we can choose
    /// not to split at all. This can be lowered using the
    /// [`with_max_len`] method, which will force us to create more
    /// parallel tasks. Note that Rayon automatically normally
    /// attempts to adjust the size of parallel splits to reduce
    /// overhead, so this should not be needed.
    ///
    /// [`with_max_len`]: ../trait.IndexedParallelIterator.html#method.with_max_len
    pub max_len: Option<usize>,
}

impl Setup {
    pub fn merge(mut self, other: Self) -> Self {
        self.splits = other.splits.or(self.splits);

        self.min_len = match (self.min_len, other.min_len) {
            (Some(a), Some(b)) => Some(max(a, b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self.max_len = match (self.max_len, other.max_len) {
            (Some(a), Some(b)) => Some(min(a, b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        self
    }
}
