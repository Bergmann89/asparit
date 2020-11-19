use std::collections::LinkedList;
use std::iter::FusedIterator;
use std::mem::replace;
use std::ops::{Range, RangeBounds};
use std::ptr::{copy, drop_in_place, read};
use std::slice::{from_raw_parts_mut, IterMut};
use std::sync::Arc;

use crate::{
    misc::simplify_range, Consumer, Executor, ExecutorCallback, Folder, FromParallelIterator,
    IndexedParallelIterator, IndexedProducer, IndexedProducerCallback, IntoParallelIterator,
    ParallelDrainRange, ParallelExtend, ParallelIterator, Producer, ProducerCallback, Reducer,
    WithIndexedProducer, WithProducer, WithSetup,
};

/* Vec */

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

impl<'a, V> ParallelExtend<'a, V::Item, VecExtendResult<V>> for V
where
    V: VecLike + Send + 'a,
{
    type Consumer = VecConsumer<V>;

    fn into_consumer(self) -> Self::Consumer {
        VecConsumer { vec: Some(self) }
    }

    fn map_result(inner: VecExtendResult<V>) -> Self {
        let mut vec = inner.vec.unwrap();

        for items in inner.items {
            vec.append(items);
        }

        vec
    }
}

impl<'a, I> FromParallelIterator<'a, I> for Vec<I>
where
    I: Send + 'a,
{
    type ExecutorItem2 = VecExtendResult<Vec<I>>;
    type ExecutorItem3 = ();

    fn from_par_iter<E, X>(executor: E, iterator: X) -> E::Result
    where
        E: Executor<'a, Self, VecExtendResult<Vec<I>>>,
        X: IntoParallelIterator<'a, Item = I>,
    {
        let result = Self::default();
        let consumer = result.into_consumer();
        let iterator = iterator.into_par_iter();

        let inner = iterator.drive(executor.into_inner(), consumer);

        E::map(inner, ParallelExtend::map_result)
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

/* IntoIter */

#[derive(Debug, Clone)]
pub struct IntoIter<T> {
    pub vec: Vec<T>,
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
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
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
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint(&self) -> usize {
        self.vec.len()
    }
}

impl<'a, T> WithProducer<'a> for IntoIter<T>
where
    T: Send + 'a,
{
    type Item = T;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(VecProducer::new(self.vec))
    }
}

impl<'a, T> WithIndexedProducer<'a> for IntoIter<T>
where
    T: Send + 'a,
{
    type Item = T;

    fn with_indexed_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        callback.callback(VecProducer::new(self.vec))
    }
}

/* VecContainer */

struct VecContainer<T>(Vec<T>);

unsafe impl<T> Sync for VecContainer<T> {}

impl<T> Drop for VecContainer<T> {
    fn drop(&mut self) {
        unsafe {
            self.0.set_len(0);
        }
    }
}

/* VecProducer */

struct VecProducer<'a, T> {
    vec: Arc<VecContainer<T>>,
    slice: &'a mut [T],
}

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

impl<'a, T> WithSetup for VecProducer<'a, T> {}

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
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
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
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint(&self) -> usize {
        self.range.len()
    }
}

impl<'a, T> WithProducer<'a> for Drain<'a, T>
where
    T: Send,
{
    type Item = T;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(self.into_producer())
    }
}

impl<'a, T> WithIndexedProducer<'a> for Drain<'a, T>
where
    T: Send,
{
    type Item = T;

    fn with_indexed_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        callback.callback(self.into_producer())
    }
}

/* DrainProducer */

struct DrainProducer<'a, T> {
    drain: Arc<Drain<'a, T>>,
    slice: &'a mut [T],
}

impl<'a, T> WithSetup for DrainProducer<'a, T> {}

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

/* VecConsumer */

pub struct VecConsumer<V> {
    vec: Option<V>,
}

impl<V> WithSetup for VecConsumer<V> {}

impl<V> Consumer<V::Item> for VecConsumer<V>
where
    V: VecLike + Send,
{
    type Folder = VecFolder<V>;
    type Reducer = VecReducer;
    type Result = VecExtendResult<V>;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let left = VecConsumer { vec: self.vec };
        let right = VecConsumer { vec: None };
        let reducer = VecReducer;

        (left, right, reducer)
    }

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        self.split()
    }

    fn into_folder(self) -> Self::Folder {
        VecFolder {
            vec: self.vec,
            items: Vec::new(),
        }
    }

    fn is_full(&self) -> bool {
        false
    }
}

/* VecFolder */

pub struct VecFolder<V: VecLike> {
    vec: Option<V>,
    items: Vec<V::Item>,
}

impl<V> Folder<V::Item> for VecFolder<V>
where
    V: VecLike,
{
    type Result = VecExtendResult<V>;

    fn consume(mut self, item: V::Item) -> Self {
        self.items.push(item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = V::Item>,
    {
        self.items.extend(iter);

        self
    }

    fn complete(self) -> Self::Result {
        let mut items = LinkedList::new();
        items.push_back(self.items);

        VecExtendResult {
            vec: self.vec,
            items,
        }
    }

    fn is_full(&self) -> bool {
        false
    }
}

/* VecReducer */

pub struct VecReducer;

impl<V> Reducer<VecExtendResult<V>> for VecReducer
where
    V: VecLike,
{
    fn reduce(self, left: VecExtendResult<V>, mut right: VecExtendResult<V>) -> VecExtendResult<V> {
        let mut items = left.items;
        items.append(&mut right.items);

        let vec = left.vec.or(right.vec);

        VecExtendResult { vec, items }
    }
}

/* VecExtendResult */

pub struct VecExtendResult<V: VecLike> {
    vec: Option<V>,
    items: LinkedList<Vec<V::Item>>,
}

/* VecLike */

pub trait VecLike {
    type Item: Send;

    fn append(&mut self, items: Vec<Self::Item>);
}

impl<I> VecLike for Vec<I>
where
    I: Send,
{
    type Item = I;

    fn append(&mut self, mut items: Vec<I>) {
        Vec::append(self, &mut items);
    }
}

impl<'a, I> VecLike for &'a mut Vec<I>
where
    I: Send,
{
    type Item = I;

    fn append(&mut self, mut items: Vec<I>) {
        Vec::append(self, &mut items);
    }
}
