use std::cmp::min;

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Producer, Reducer, Setup, WithIndexedProducer,
    WithSetup,
};

pub struct Zip<XA, XB> {
    iterator_a: XA,
    iterator_b: XB,
}

impl<XA, XB> Zip<XA, XB> {
    pub fn new(iterator_a: XA, iterator_b: XB) -> Self {
        Self {
            iterator_a,
            iterator_b,
        }
    }
}

impl<'a, XA, XB, A, B> ParallelIterator<'a> for Zip<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = A> + WithIndexedProducer<'a, Item = A>,
    XB: IndexedParallelIterator<'a, Item = B> + WithIndexedProducer<'a, Item = B>,
    A: Send + 'a,
    B: Send + 'a,
{
    type Item = (A, B);

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
        match (
            self.iterator_a.len_hint_opt(),
            self.iterator_b.len_hint_opt(),
        ) {
            (Some(a), Some(b)) => Some(min(a, b)),
            (_, _) => None,
        }
    }
}

impl<'a, XA, XB, A, B> IndexedParallelIterator<'a> for Zip<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = A> + WithIndexedProducer<'a, Item = A>,
    XB: IndexedParallelIterator<'a, Item = B> + WithIndexedProducer<'a, Item = B>,
    A: Send + 'a,
    B: Send + 'a,
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
        min(self.iterator_a.len_hint(), self.iterator_b.len_hint())
    }
}

impl<'a, XA, XB> WithIndexedProducer<'a> for Zip<XA, XB>
where
    XA: WithIndexedProducer<'a>,
    XB: WithIndexedProducer<'a>,
{
    type Item = (XA::Item, XB::Item);

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        let iterator_a = self.iterator_a;
        let iterator_b = self.iterator_b;

        iterator_a.with_indexed_producer(CallbackA { base, iterator_b })
    }
}

/* CallbackA */

struct CallbackA<CB, XB> {
    base: CB,
    iterator_b: XB,
}

impl<'a, CB, XB, A, B> IndexedProducerCallback<'a, A> for CallbackA<CB, XB>
where
    CB: IndexedProducerCallback<'a, (A, B)>,
    XB: WithIndexedProducer<'a, Item = B>,
    A: Send + 'a,
    B: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_a: P) -> Self::Output
    where
        P: IndexedProducer<Item = A> + 'a,
    {
        let CallbackA { base, iterator_b } = self;

        iterator_b.with_indexed_producer(CallbackB { base, producer_a })
    }
}

/* CallbackB */

struct CallbackB<CB, PA> {
    base: CB,
    producer_a: PA,
}

impl<'a, CB, PA, A, B> IndexedProducerCallback<'a, B> for CallbackB<CB, PA>
where
    CB: IndexedProducerCallback<'a, (A, B)>,
    PA: IndexedProducer<Item = A> + 'a,
    A: Send + 'a,
    B: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_b: P) -> Self::Output
    where
        P: IndexedProducer<Item = B> + 'a,
    {
        let CallbackB { base, producer_a } = self;

        let producer = ZipProducer {
            producer_a,
            producer_b,
        };

        base.callback(producer)
    }
}

/* ZipProducer */

struct ZipProducer<PA, PB> {
    producer_a: PA,
    producer_b: PB,
}

impl<PA, PB> WithSetup for ZipProducer<PA, PB>
where
    PA: WithSetup,
    PB: WithSetup,
{
    fn setup(&self) -> Setup {
        let a = self.producer_a.setup();
        let b = self.producer_b.setup();

        a.merge(b)
    }
}

impl<PA, PB> Producer for ZipProducer<PA, PB>
where
    PA: IndexedProducer,
    PB: IndexedProducer,
{
    type Item = (PA::Item, PB::Item);
    type IntoIter = std::iter::Zip<PA::IntoIter, PB::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let a = self.producer_a.into_iter();
        let b = self.producer_b.into_iter();

        a.zip(b)
    }

    fn split(self) -> (Self, Option<Self>) {
        let len = self.len();

        if len < 2 {
            return (self, None);
        }

        let index = len / 2;
        let (left_a, right_a) = self.producer_a.split_at(index);
        let (left_b, right_b) = self.producer_b.split_at(index);

        let left = Self {
            producer_a: left_a,
            producer_b: left_b,
        };
        let right = Self {
            producer_a: right_a,
            producer_b: right_b,
        };

        (left, Some(right))
    }
}

impl<PA, PB> IndexedProducer for ZipProducer<PA, PB>
where
    PA: IndexedProducer,
    PB: IndexedProducer,
{
    type Item = (PA::Item, PB::Item);
    type IntoIter = std::iter::Zip<PA::IntoIter, PB::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        let a = self.producer_a.into_iter();
        let b = self.producer_b.into_iter();

        a.zip(b)
    }

    fn len(&self) -> usize {
        min(self.producer_a.len(), self.producer_b.len())
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left_a, right_a) = self.producer_a.split_at(index);
        let (left_b, right_b) = self.producer_b.split_at(index);

        let left = Self {
            producer_a: left_a,
            producer_b: left_b,
        };
        let right = Self {
            producer_a: right_a,
            producer_b: right_b,
        };

        (left, right)
    }
}
