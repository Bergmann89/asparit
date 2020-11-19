use std::iter::Zip;
use std::ops::Range;

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Producer, Reducer, Setup, WithIndexedProducer,
    WithSetup,
};

pub struct Enumerate<X> {
    base: X,
}

impl<X> Enumerate<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X, I> ParallelIterator<'a> for Enumerate<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Item = (usize, I);

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
        self.base.len_hint_opt()
    }
}

impl<'a, X, I> IndexedParallelIterator<'a> for Enumerate<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
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
        self.base.len_hint()
    }
}

impl<'a, X> WithIndexedProducer<'a> for Enumerate<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = (usize, X::Item);

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(EnumerateCallback { base })
    }
}

/* EnumerateCallback */

struct EnumerateCallback<CB> {
    base: CB,
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for EnumerateCallback<CB>
where
    CB: IndexedProducerCallback<'a, (usize, I)>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        self.base.callback(EnumerateProducer { base, offset: 0 })
    }
}

/* EnumerateProducer */

struct EnumerateProducer<P> {
    base: P,
    offset: usize,
}

impl<P> WithSetup for EnumerateProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<P> Producer for EnumerateProducer<P>
where
    P: IndexedProducer,
{
    type Item = (usize, P::Item);
    type IntoIter = Zip<Range<usize>, P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let base = self.base.into_iter();
        let start = self.offset;
        let end = start + base.len();

        (start..end).zip(base)
    }

    fn split(self) -> (Self, Option<Self>) {
        let len = self.base.len();
        if len < 2 {
            return (self, None);
        }

        let (left, right) = self.split_at(len / 2);

        (left, Some(right))
    }
}

impl<P> IndexedProducer for EnumerateProducer<P>
where
    P: IndexedProducer,
{
    type Item = (usize, P::Item);
    type IntoIter = Zip<Range<usize>, P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let base = self.base.into_iter();
        let start = self.offset;
        let end = start + base.len();

        (start..end).zip(base)
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = Self {
            base: left,
            offset: self.offset,
        };
        let right = Self {
            base: right,
            offset: self.offset + index,
        };

        (left, right)
    }
}
