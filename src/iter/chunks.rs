use std::cmp::min;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Reducer, Setup, WithIndexedProducer, WithSetup,
};

pub struct Chunks<X> {
    base: X,
    size: usize,
}

impl<X> Chunks<X> {
    pub fn new(base: X, size: usize) -> Self {
        Self { base, size }
    }
}

impl<'a, X, I> ParallelIterator<'a> for Chunks<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Item = Vec<I>;

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
        Some(self.len_hint())
    }
}

impl<'a, X, I> IndexedParallelIterator<'a> for Chunks<X>
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
        let mut len = self.base.len_hint();

        if len > 0 {
            len = (len - 1) / self.size - 1;
        }

        len
    }
}

impl<'a, X> WithIndexedProducer<'a> for Chunks<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = Vec<X::Item>;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(ChunksCallback {
            base,
            size: self.size,
        })
    }
}

/* ChunksCallback */

struct ChunksCallback<CB> {
    base: CB,
    size: usize,
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for ChunksCallback<CB>
where
    CB: IndexedProducerCallback<'a, Vec<I>>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        self.base.callback(ChunkProducer {
            base,
            size: self.size,
        })
    }
}

/* ChunkProducer */

struct ChunkProducer<P> {
    base: P,
    size: usize,
}

impl<P> WithSetup for ChunkProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<P> IndexedProducer for ChunkProducer<P>
where
    P: IndexedProducer,
{
    type Item = Vec<P::Item>;
    type IntoIter = ChunksIter<P>;

    fn into_iter(self) -> Self::IntoIter {
        ChunksIter {
            producer: if self.len() > 0 {
                Some(self.base)
            } else {
                None
            },
            size: self.size,
        }
    }

    fn len(&self) -> usize {
        let len = self.base.len();

        if len > 0 {
            (len - 1) / self.size + 1
        } else {
            0
        }
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let index = min(index * self.size, self.base.len());
        let (left, right) = self.base.split_at(index);

        let left = Self {
            base: left,
            size: self.size,
        };
        let right = Self {
            base: right,
            size: self.size,
        };

        (left, right)
    }
}

/* ChunksIter */

struct ChunksIter<P> {
    producer: Option<P>,
    size: usize,
}

impl<P> Iterator for ChunksIter<P>
where
    P: IndexedProducer,
{
    type Item = Vec<P::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let producer = self.producer.take()?;
        let producer = if producer.len() > self.size {
            let index = self.size;
            let (left, right) = producer.split_at(index);

            self.producer = Some(right);

            left
        } else {
            producer
        };

        Some(producer.into_iter().collect())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();

        (len, Some(len))
    }
}

impl<P> DoubleEndedIterator for ChunksIter<P>
where
    P: IndexedProducer,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let producer = self.producer.take()?;
        let producer = if producer.len() > self.size {
            let mut size = producer.len() % self.size;
            if size == 0 {
                size = self.size;
            }

            let index = producer.len() - size;
            let (left, right) = producer.split_at(index);

            self.producer = Some(left);

            right
        } else {
            producer
        };

        Some(producer.into_iter().collect())
    }
}

impl<P> ExactSizeIterator for ChunksIter<P>
where
    P: IndexedProducer,
{
    #[inline]
    fn len(&self) -> usize {
        let len = self
            .producer
            .as_ref()
            .map(IndexedProducer::len)
            .unwrap_or_default();

        if len > 0 {
            (len - 1) / self.size + 1
        } else {
            0
        }
    }
}
