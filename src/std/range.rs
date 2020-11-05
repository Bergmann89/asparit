use std::ops::Range;

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedConsumer, IndexedParallelIterator,
    IndexedProducer, IndexedProducerCallback, IntoParallelIterator, ParallelIterator, Producer,
    ProducerCallback, Reducer,
};

/// Parallel iterator over a range, implemented for all integer types.
///
/// **Note:** The `zip` operation requires `IndexedParallelIterator`
/// which is not implemented for `u64`, `i64`, `u128`, or `i128`.
///
/// ```
/// use rayon::prelude::*;
///
/// let p = (0..25usize)
///     .into_par_iter()
///     .zip(0..25usize)
///     .filter(|&(x, y)| x % 5 == 0 || y % 5 == 0)
///     .map(|(x, y)| x * y)
///     .sum::<usize>();
///
/// let s = (0..25usize)
///     .zip(0..25)
///     .filter(|&(x, y)| x % 5 == 0 || y % 5 == 0)
///     .map(|(x, y)| x * y)
///     .sum();
///
/// assert_eq!(p, s);
/// ```
#[derive(Debug, Clone)]
pub struct Iter<T> {
    range: Range<T>,
}

struct IterProducer<T> {
    range: Range<T>,
}

trait RangeLen<L> {
    fn length(&self) -> L;
}

impl<'a, T> IntoParallelIterator<'a> for Range<T>
where
    Iter<T>: ParallelIterator<'a>,
{
    type Item = <Iter<T> as ParallelIterator<'a>>::Item;
    type Iter = Iter<T>;

    fn into_par_iter(self) -> Self::Iter {
        Iter { range: self }
    }
}

impl<T> IntoIterator for IterProducer<T>
where
    Range<T>: Iterator,
{
    type Item = <Range<T> as Iterator>::Item;
    type IntoIter = Range<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.range
    }
}

macro_rules! range_len_impl {
    ( $t:ty, $len_t:ty ) => {
        impl RangeLen<$len_t> for Range<$t> {
            fn length(&self) -> $len_t {
                if self.start < self.end {
                    self.end.wrapping_sub(self.start) as $len_t
                } else {
                    0
                }
            }
        }
    };
}

macro_rules! unindexed_parallel_iterator_impl {
    ( $t:ty, $len_t:ty ) => {
        range_len_impl!($t, $len_t);

        impl<'a> ParallelIterator<'a> for Iter<$t> {
            type Item = $t;

            fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
            where
                E: Executor<'a, D>,
                C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
                D: Send,
                R: Reducer<D> + Send,
            {
                self.with_producer(ExecutorCallback::new(executor, consumer))
            }

            fn with_producer<CB>(self, callback: CB) -> CB::Output
            where
                CB: ProducerCallback<'a, Self::Item>,
            {
                callback.callback(IterProducer { range: self.range })
            }

            fn len_hint_opt(&self) -> Option<usize> {
                Some(self.range.length() as usize)
            }
        }

        impl Producer for IterProducer<$t> {
            type Item = $t;
            type IntoIter = Range<$t>;

            fn into_iter(self) -> Self::IntoIter {
                self.range
            }

            fn split(mut self) -> (Self, Option<Self>) {
                let index = self.range.length() / 2;

                if index > 0 {
                    let mid = self.range.start.wrapping_add(index as $t);
                    let right = mid..self.range.end;

                    self.range.end = mid;

                    (self, Some(IterProducer { range: right }))
                } else {
                    (self, None)
                }
            }
        }
    };
}

macro_rules! indexed_parallel_iterator_impl {
    ( $t:ty, $len_t:ty ) => {
        unindexed_parallel_iterator_impl!($t, $len_t);

        impl<'a> IndexedParallelIterator<'a> for Iter<$t> {
            fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
            where
                E: Executor<'a, D>,
                C: IndexedConsumer<Self::Item, Result = D, Reducer = R> + 'a,
                D: Send,
                R: Reducer<D> + Send,
            {
                self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
            }

            fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
            where
                CB: IndexedProducerCallback<'a, Self::Item>,
            {
                callback.callback(IterProducer { range: self.range })
            }

            fn len_hint(&self) -> usize {
                self.range.length() as usize
            }
        }

        impl IndexedProducer for IterProducer<$t> {
            type Item = $t;
            type IntoIter = Range<$t>;

            fn into_iter(self) -> Self::IntoIter {
                self.range
            }

            fn len(&self) -> usize {
                self.range.length() as usize
            }

            fn split_at(self, index: usize) -> (Self, Self) {
                assert!(index <= self.range.length() as usize);

                let mid = self.range.start.wrapping_add(index as $t);
                let left = self.range.start..mid;
                let right = mid..self.range.end;

                (IterProducer { range: left }, IterProducer { range: right })
            }
        }
    };
}

indexed_parallel_iterator_impl!(u8, u8);
indexed_parallel_iterator_impl!(i8, u8);
indexed_parallel_iterator_impl!(u16, u16);
indexed_parallel_iterator_impl!(i16, u16);
indexed_parallel_iterator_impl!(u32, u32);
indexed_parallel_iterator_impl!(i32, u32);
indexed_parallel_iterator_impl!(usize, usize);
indexed_parallel_iterator_impl!(isize, usize);

unindexed_parallel_iterator_impl!(u64, u64);
unindexed_parallel_iterator_impl!(i64, u64);
unindexed_parallel_iterator_impl!(u128, u128);
unindexed_parallel_iterator_impl!(i128, u128);
