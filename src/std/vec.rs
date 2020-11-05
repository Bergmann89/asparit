use std::iter::FusedIterator;
use std::mem::replace;
use std::ops::{Range, RangeBounds};
use std::ptr::{copy, drop_in_place, read};
use std::slice::{from_raw_parts_mut, IterMut};
use std::sync::Arc;

use crate::{
    misc::simplify_range, Consumer, Executor, ExecutorCallback, IndexedParallelIterator,
    IndexedProducer, IndexedProducerCallback, IntoParallelIterator, ParallelDrainRange,
    ParallelIterator, Producer, ProducerCallback, Reducer,
};

/// Parallel iterator that moves out of a vector.
#[derive(Debug, Clone)]
pub struct IntoIter<T: Send> {
    vec: Vec<T>,
}

impl<'a, T> IntoParallelIterator<'a> for Vec<T>
where
    T: Send + 'a,
{
    type Item = T;
    type Iter = IntoIter<T>;

    fn into_par_iter(self) -> Self::Iter {
        IntoIter { vec: self }
    }
}

impl<'a, T> ParallelIterator<'a> for IntoIter<T>
where
    T: Send + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(VecProducer::new(self.vec))
    }

    fn len_hint_opt(&self) -> Option<usize> {
        Some(self.vec.len())
    }
}

impl<'a, T> IndexedParallelIterator<'a> for IntoIter<T>
where
    T: Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        callback.callback(VecProducer::new(self.vec))
    }

    fn len_hint(&self) -> usize {
        self.vec.len()
    }
}

impl<'a, T> ParallelDrainRange<'a, usize> for &'a mut Vec<T>
where
    T: Send,
{
    type Iter = Drain<'a, T>;
    type Item = T;

    fn par_drain<R: RangeBounds<usize>>(self, range: R) -> Self::Iter {
        let length = self.len();

        Drain {
            vec: self,
            range: simplify_range(range, length),
            length,
        }
    }
}

/* VecProducer */

struct VecProducer<'a, T> {
    vec: Arc<VecContainer<T>>,
    slice: &'a mut [T],
}

struct VecContainer<T>(Vec<T>);

impl<T> Drop for VecContainer<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.set_len(0);
        }
    }
}

unsafe impl<T> Sync for VecContainer<T> {}

impl<'a, T> VecProducer<'a, T> {
    fn new(mut vec: Vec<T>) -> Self {
        unsafe {
            let len = vec.len();
            let ptr = vec.as_mut_ptr();
            let slice = from_raw_parts_mut(ptr, len);

            Self {
                vec: Arc::new(VecContainer(vec)),
                slice,
            }
        }
    }
}

impl<'a, T> Drop for VecProducer<'a, T> {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(self.slice);
        }
    }
}

impl<'a, T> Producer for VecProducer<'a, T>
where
    T: Send,
{
    type Item = T;
    type IntoIter = SliceIter<'a, T, Arc<VecContainer<T>>>;

    fn into_iter(mut self) -> Self::IntoIter {
        // replace the slice so we don't drop it twice
        let slice = replace(&mut self.slice, &mut []);

        SliceIter {
            _container: self.vec.clone(),
            iter: slice.iter_mut(),
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        if self.slice.len() < 2 {
            (self, None)
        } else {
            let mid = self.slice.len() / 2;
            let (left, right) = self.split_at(mid);

            (left, Some(right))
        }
    }
}

impl<'a, T> IndexedProducer for VecProducer<'a, T>
where
    T: Send,
{
    type Item = T;
    type IntoIter = SliceIter<'a, T, Arc<VecContainer<T>>>;

    fn into_iter(mut self) -> Self::IntoIter {
        // replace the slice so we don't drop it twice
        let slice = replace(&mut self.slice, &mut []);

        SliceIter {
            _container: self.vec.clone(),
            iter: slice.iter_mut(),
        }
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn split_at(mut self, index: usize) -> (Self, Self) {
        // replace the slice so we don't drop it twice
        let slice = replace(&mut self.slice, &mut []);
        let (left, right) = slice.split_at_mut(index);

        let left = VecProducer {
            vec: self.vec.clone(),
            slice: left,
        };
        let right = VecProducer {
            vec: self.vec.clone(),
            slice: right,
        };

        (left, right)
    }
}

/* Drain */

#[derive(Debug)]
pub struct Drain<'a, T> {
    vec: &'a mut Vec<T>,
    range: Range<usize>,
    length: usize,
}

impl<'a, T> Drain<'a, T>
where
    Self: 'a,
{
    fn into_producer(self) -> DrainProducer<'a, T> {
        unsafe {
            let mut drain = Arc::new(self);
            let this = Arc::get_mut(&mut drain).unwrap();

            // Make the vector forget about the drained items, and temporarily the tail too.
            let start = this.range.start;
            this.vec.set_len(start);

            // Get slice of the processed data.
            let ptr_start = this.vec.as_mut_ptr().add(start);
            let slice = from_raw_parts_mut(ptr_start, this.range.len());

            DrainProducer { drain, slice }
        }
    }
}

unsafe impl<'a, T> Sync for Drain<'a, T> {}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        if self.range.is_empty() {
            return;
        }

        let Range { start, end } = self.range;
        if self.vec.len() != start {
            assert_eq!(self.vec.len(), self.length);

            self.vec.drain(start..end);
        } else if end < self.length {
            unsafe {
                let ptr_start = self.vec.as_mut_ptr().add(start);
                let ptr_end = self.vec.as_ptr().add(end);
                let count = self.length - end;

                copy(ptr_end, ptr_start, count);

                self.vec.set_len(start + count);
            }
        }
    }
}

impl<'a, T> ParallelIterator<'a> for Drain<'a, T>
where
    T: Send,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(self.into_producer())
    }

    fn len_hint_opt(&self) -> Option<usize> {
        Some(self.range.len())
    }
}

impl<'a, T> IndexedParallelIterator<'a> for Drain<'a, T>
where
    T: Send,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        callback.callback(self.into_producer())
    }

    fn len_hint(&self) -> usize {
        self.range.len()
    }
}

/* DrainProducer */

struct DrainProducer<'a, T> {
    drain: Arc<Drain<'a, T>>,
    slice: &'a mut [T],
}

impl<'a, T> Drop for DrainProducer<'a, T> {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(self.slice);
        }
    }
}

impl<'a, T> Producer for DrainProducer<'a, T>
where
    T: Send,
{
    type Item = T;
    type IntoIter = SliceIter<'a, T, Arc<Drain<'a, T>>>;

    fn into_iter(mut self) -> Self::IntoIter {
        // replace the slice so we don't drop it twice
        let slice = replace(&mut self.slice, &mut []);

        SliceIter {
            _container: self.drain.clone(),
            iter: slice.iter_mut(),
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        if self.slice.len() < 2 {
            (self, None)
        } else {
            let mid = self.slice.len() / 2;
            let (left, right) = self.split_at(mid);

            (left, Some(right))
        }
    }
}

impl<'a, T> IndexedProducer for DrainProducer<'a, T>
where
    T: Send,
{
    type Item = T;
    type IntoIter = SliceIter<'a, T, Arc<Drain<'a, T>>>;

    fn into_iter(mut self) -> Self::IntoIter {
        // replace the slice so we don't drop it twice
        let slice = replace(&mut self.slice, &mut []);

        SliceIter {
            _container: self.drain.clone(),
            iter: slice.iter_mut(),
        }
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn split_at(mut self, index: usize) -> (Self, Self) {
        // replace the slice so we don't drop it twice
        let slice = replace(&mut self.slice, &mut []);
        let (left, right) = slice.split_at_mut(index);

        let left = DrainProducer {
            slice: left,
            drain: self.drain.clone(),
        };
        let right = DrainProducer {
            slice: right,
            drain: self.drain.clone(),
        };

        (left, right)
    }
}

/* SliceIter */

struct SliceIter<'a, T, C> {
    _container: C,
    iter: IterMut<'a, T>,
}

impl<'a, T, C> Drop for SliceIter<'a, T, C> {
    fn drop(&mut self) {
        // extract the iterator so we can use `Drop for [T]`
        let iter = replace(&mut self.iter, [].iter_mut());

        unsafe { drop_in_place(iter.into_slice()) };
    }
}

impl<'a, T, C> Iterator for SliceIter<'a, T, C> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let ptr = self.iter.next()?;

        Some(unsafe { read(ptr) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.len()
    }
}

impl<'a, T, C> DoubleEndedIterator for SliceIter<'a, T, C> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ptr = self.iter.next_back()?;

        Some(unsafe { read(ptr) })
    }
}

impl<'a, T, C> ExactSizeIterator for SliceIter<'a, T, C> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, T, C> FusedIterator for SliceIter<'a, T, C> {}
