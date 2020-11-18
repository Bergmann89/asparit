use std::cmp::min;

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Reducer, WithIndexedProducer,
};

pub struct Take<X> {
    base: X,
    len: usize,
}

impl<X> Take<X> {
    pub fn new(base: X, len: usize) -> Self {
        Self { base, len }
    }
}

impl<'a, X, I> ParallelIterator<'a> for Take<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Item = I;

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
        self.base.len_hint_opt().map(|len| min(len, self.len))
    }
}

impl<'a, X, I> IndexedParallelIterator<'a> for Take<X>
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
        min(self.base.len_hint(), self.len)
    }
}

impl<'a, X> WithIndexedProducer<'a> for Take<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = X::Item;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(TakeCallback {
            base,
            len: self.len,
        })
    }
}

/* TakeCallback */

struct TakeCallback<CB> {
    base: CB,
    len: usize,
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for TakeCallback<CB>
where
    CB: IndexedProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let len = dbg!(min(self.len, producer.len()));
        let (producer, _) = producer.split_at(len);

        self.base.callback(producer)
    }
}
