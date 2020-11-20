use std::cmp;

/* Splitter */

#[derive(Clone, Copy, Debug)]
pub struct Splitter {
    splits: usize,
}

impl Splitter {
    #[inline]
    pub fn new(splits: usize) -> Self {
        Self { splits }
    }

    #[inline]
    pub fn try_split(&mut self) -> bool {
        if self.splits > 1 {
            self.splits /= 2;

            true
        } else {
            false
        }
    }
}

/* IndexedSplitter */

#[derive(Clone, Copy, Debug)]
pub struct IndexedSplitter {
    inner: Splitter,
    min: usize,
}

impl IndexedSplitter {
    #[inline]
    pub fn new(splits: usize, len: usize, min: Option<usize>, max: Option<usize>) -> Self {
        let min = min.unwrap_or_default();
        let mut ret = Self {
            inner: Splitter::new(splits),
            min: cmp::max(min, 1),
        };

        if let Some(max) = max {
            let mut min_splits = len / cmp::max(max, 1);

            // Calculate next Power of Two
            min_splits -= 1;
            min_splits |= min_splits >> 1;
            min_splits |= min_splits >> 2;
            min_splits |= min_splits >> 4;
            min_splits |= min_splits >> 8;
            min_splits |= min_splits >> 16;
            min_splits += 1;

            if min_splits > ret.inner.splits {
                ret.inner.splits = min_splits;
            }
        }

        ret
    }

    #[inline]
    pub fn try_split(&mut self, len: usize) -> bool {
        len / 2 >= self.min && self.inner.try_split()
    }
}
