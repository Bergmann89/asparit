mod try_;

pub use try_::Try;

use std::ops::{Bound, Range, RangeBounds};

pub fn simplify_range(range: impl RangeBounds<usize>, len: usize) -> Range<usize> {
    let start = match range.start_bound() {
        Bound::Unbounded => 0,
        Bound::Included(&i) if i <= len => i,
        Bound::Excluded(&i) if i < len => i + 1,
        bound => panic!("range start {:?} should be <= length {}", bound, len),
    };

    let end = match range.end_bound() {
        Bound::Unbounded => len,
        Bound::Excluded(&i) if i <= len => i,
        Bound::Included(&i) if i < len => i + 1,
        bound => panic!("range end {:?} should be <= length {}", bound, len),
    };

    if start > end {
        panic!(
            "range start {:?} should be <= range end {:?}",
            range.start_bound(),
            range.end_bound()
        );
    }

    start..end
}
