use std::ops::RangeBounds;

use crate::ParallelIterator;

/// `ParallelDrainFull` creates a parallel iterator that moves all items
/// from a collection while retaining the original capacity.
///
/// Types which are indexable typically implement [`ParallelDrainRange`]
/// instead, where you can drain fully with `par_drain(..)`.
///
/// [`ParallelDrainRange`]: trait.ParallelDrainRange.html
pub trait ParallelDrainFull<'a> {
    /// The draining parallel iterator type that will be created.
    type Iter: ParallelIterator<'a, Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This is usually the same as `IntoParallelIterator::Item`.
    type Item: Send;

    /// Returns a draining parallel iterator over an entire collection.
    ///
    /// When the iterator is dropped, all items are removed, even if the
    /// iterator was not fully consumed. If the iterator is leaked, for example
    /// using `std::mem::forget`, it is unspecified how many items are removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    /// use std::collections::{BinaryHeap, HashSet};
    ///
    /// let squares: HashSet<i32> = (0..10).map(|x| x * x).collect();
    ///
    /// let mut heap: BinaryHeap<_> = squares.iter().copied().collect();
    /// assert_eq!(
    ///     // heaps are drained in arbitrary order
    ///     heap.par_drain()
    ///         .inspect(|x| assert!(squares.contains(x)))
    ///         .count(),
    ///     squares.len(),
    /// );
    /// assert!(heap.is_empty());
    /// assert!(heap.capacity() >= squares.len());
    /// ```
    fn par_drain(self) -> Self::Iter;
}

/// `ParallelDrainRange` creates a parallel iterator that moves a range of items
/// from a collection while retaining the original capacity.
///
/// Types which are not indexable may implement [`ParallelDrainFull`] instead.
///
/// [`ParallelDrainFull`]: trait.ParallelDrainFull.html
pub trait ParallelDrainRange<'a, Idx = usize> {
    /// The draining parallel iterator type that will be created.
    type Iter: ParallelIterator<'a, Item = Self::Item>;

    /// The type of item that the parallel iterator will produce.
    /// This is usually the same as `IntoParallelIterator::Item`.
    type Item: Send;

    /// Returns a draining parallel iterator over a range of the collection.
    ///
    /// When the iterator is dropped, all items in the range are removed, even
    /// if the iterator was not fully consumed. If the iterator is leaked, for
    /// example using `std::mem::forget`, it is unspecified how many items are
    /// removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rayon::prelude::*;
    ///
    /// let squares: Vec<i32> = (0..10).map(|x| x * x).collect();
    ///
    /// println!("RangeFull");
    /// let mut vec = squares.clone();
    /// assert!(vec.par_drain(..)
    ///            .eq(squares.par_iter().copied()));
    /// assert!(vec.is_empty());
    /// assert!(vec.capacity() >= squares.len());
    ///
    /// println!("RangeFrom");
    /// let mut vec = squares.clone();
    /// assert!(vec.par_drain(5..)
    ///            .eq(squares[5..].par_iter().copied()));
    /// assert_eq!(&vec[..], &squares[..5]);
    /// assert!(vec.capacity() >= squares.len());
    ///
    /// println!("RangeTo");
    /// let mut vec = squares.clone();
    /// assert!(vec.par_drain(..5)
    ///            .eq(squares[..5].par_iter().copied()));
    /// assert_eq!(&vec[..], &squares[5..]);
    /// assert!(vec.capacity() >= squares.len());
    ///
    /// println!("RangeToInclusive");
    /// let mut vec = squares.clone();
    /// assert!(vec.par_drain(..=5)
    ///            .eq(squares[..=5].par_iter().copied()));
    /// assert_eq!(&vec[..], &squares[6..]);
    /// assert!(vec.capacity() >= squares.len());
    ///
    /// println!("Range");
    /// let mut vec = squares.clone();
    /// assert!(vec.par_drain(3..7)
    ///            .eq(squares[3..7].par_iter().copied()));
    /// assert_eq!(&vec[..3], &squares[..3]);
    /// assert_eq!(&vec[3..], &squares[7..]);
    /// assert!(vec.capacity() >= squares.len());
    ///
    /// println!("RangeInclusive");
    /// let mut vec = squares.clone();
    /// assert!(vec.par_drain(3..=7)
    ///            .eq(squares[3..=7].par_iter().copied()));
    /// assert_eq!(&vec[..3], &squares[..3]);
    /// assert_eq!(&vec[3..], &squares[8..]);
    /// assert!(vec.capacity() >= squares.len());
    /// ```
    fn par_drain<R: RangeBounds<Idx>>(self, range: R) -> Self::Iter;
}
