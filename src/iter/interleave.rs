use std::cmp::Ordering;
use std::iter::{DoubleEndedIterator, ExactSizeIterator, Fuse, Iterator};

use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Producer, Reducer, Setup, WithIndexedProducer,
    WithSetup,
};

pub struct Interleave<XA, XB> {
    iterator_a: XA,
    iterator_b: XB,
}

impl<XA, XB> Interleave<XA, XB> {
    pub fn new(iterator_a: XA, iterator_b: XB) -> Self {
        Self {
            iterator_a,
            iterator_b,
        }
    }
}

impl<'a, XA, XB, I> ParallelIterator<'a> for Interleave<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    XB: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
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
        Some(self.len_hint())
    }
}

impl<'a, XA, XB, I> IndexedParallelIterator<'a> for Interleave<XA, XB>
where
    XA: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    XB: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
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
        self.iterator_a.len_hint() + self.iterator_b.len_hint()
    }
}

impl<'a, XA, XB, I> WithIndexedProducer<'a> for Interleave<XA, XB>
where
    XA: WithIndexedProducer<'a, Item = I>,
    XB: WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Item = I;

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

impl<'a, CB, XB, I> IndexedProducerCallback<'a, I> for CallbackA<CB, XB>
where
    CB: IndexedProducerCallback<'a, I>,
    XB: WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_a: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
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

impl<'a, CB, PA, I> IndexedProducerCallback<'a, I> for CallbackB<CB, PA>
where
    CB: IndexedProducerCallback<'a, I>,
    PA: IndexedProducer<Item = I> + 'a,
    I: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_b: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let CallbackB { base, producer_a } = self;

        let producer = InterleaveProducer {
            producer_a,
            producer_b,
            a_is_next: false,
        };

        base.callback(producer)
    }
}

/* InterleaveProducer */

struct InterleaveProducer<PA, PB> {
    producer_a: PA,
    producer_b: PB,
    a_is_next: bool,
}

impl<PA, PB> WithSetup for InterleaveProducer<PA, PB>
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

impl<PA, PB, I> Producer for InterleaveProducer<PA, PB>
where
    PA: IndexedProducer<Item = I>,
    PB: IndexedProducer<Item = I>,
{
    type Item = I;
    type IntoIter = InterleaveIter<PA::IntoIter, PB::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        InterleaveIter {
            iter_a: self.producer_a.into_iter().fuse(),
            iter_b: self.producer_b.into_iter().fuse(),
            a_is_next: self.a_is_next,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let len = self.len();

        if len < 2 {
            return (self, None);
        }

        let (left, right) = self.split_at(len / 2);

        (left, Some(right))
    }
}

impl<PA, PB, I> IndexedProducer for InterleaveProducer<PA, PB>
where
    PA: IndexedProducer<Item = I>,
    PB: IndexedProducer<Item = I>,
{
    type Item = I;
    type IntoIter = InterleaveIter<PA::IntoIter, PB::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        InterleaveIter {
            iter_a: self.producer_a.into_iter().fuse(),
            iter_b: self.producer_b.into_iter().fuse(),
            a_is_next: self.a_is_next,
        }
    }

    fn len(&self) -> usize {
        self.producer_a.len() + self.producer_b.len()
    }

    /// We know 0 < index <= self.producer_a.len() + self.producer_a.len()
    ///
    /// Find a, b satisfying:
    ///
    ///  (1) 0 < a <= self.producer_a.len()
    ///  (2) 0 < b <= self.producer_b.len()
    ///  (3) a + b == index
    ///
    /// For even splits, set a = b = index/2.
    /// For odd splits, set a = (index / 2) + 1, b = index / 2, if `a`
    /// should yield the next element, otherwise, if `b` should yield
    /// the next element, set a = index / 2 and b = (index/2) + 1
    fn split_at(self, index: usize) -> (Self, Self) {
        #[inline]
        fn odd_offset(flag: bool) -> usize {
            (!flag) as usize
        }

        let even = index % 2 == 0;
        let idx = index >> 1;

        let (index_a, index_b) = (
            idx + odd_offset(even || self.a_is_next),
            idx + odd_offset(even || !self.a_is_next),
        );

        let (index_a, index_b) =
            if self.producer_a.len() >= index_a && self.producer_b.len() >= index_b {
                (index_a, index_b)
            } else if self.producer_a.len() >= index_a {
                (index - self.producer_b.len(), self.producer_b.len())
            } else {
                (self.producer_a.len(), index - self.producer_a.len())
            };

        let trailing_a_is_next = even == self.a_is_next;
        let (left_a, right_a) = self.producer_a.split_at(index_a);
        let (left_b, right_b) = self.producer_b.split_at(index_b);

        let left = Self {
            producer_a: left_a,
            producer_b: left_b,
            a_is_next: self.a_is_next,
        };
        let right = Self {
            producer_a: right_a,
            producer_b: right_b,
            a_is_next: trailing_a_is_next,
        };

        (left, right)
    }
}

/* InterleaveIter */

struct InterleaveIter<IA, IB> {
    iter_a: Fuse<IA>,
    iter_b: Fuse<IB>,
    a_is_next: bool,
}

impl<IA, IB, I> Iterator for InterleaveIter<IA, IB>
where
    IA: Iterator<Item = I>,
    IB: Iterator<Item = I>,
{
    type Item = I;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.a_is_next = !self.a_is_next;

        if self.a_is_next {
            match self.iter_a.next() {
                None => self.iter_b.next(),
                r => r,
            }
        } else {
            match self.iter_b.next() {
                None => self.iter_a.next(),
                r => r,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min_a, max_a) = self.iter_a.size_hint();
        let (min_b, max_b) = self.iter_b.size_hint();

        let min = min_a.saturating_add(min_b);
        let max = match (max_a, max_b) {
            (Some(a), Some(b)) => a.checked_add(b),
            _ => None,
        };

        (min, max)
    }
}

impl<IA, IB, I> DoubleEndedIterator for InterleaveIter<IA, IB>
where
    IA: DoubleEndedIterator<Item = I> + ExactSizeIterator<Item = I>,
    IB: DoubleEndedIterator<Item = I> + ExactSizeIterator<Item = I>,
{
    #[inline]
    fn next_back(&mut self) -> Option<I> {
        let len_a = self.iter_a.len();
        let len_b = self.iter_b.len();

        match len_a.cmp(&len_b) {
            Ordering::Less => self.iter_a.next_back(),
            Ordering::Greater => self.iter_b.next_back(),
            Ordering::Equal => {
                if self.a_is_next {
                    self.iter_a.next_back()
                } else {
                    self.iter_b.next_back()
                }
            }
        }
    }
}

impl<IA, IB, I> ExactSizeIterator for InterleaveIter<IA, IB>
where
    IA: ExactSizeIterator<Item = I>,
    IB: ExactSizeIterator<Item = I>,
{
    #[inline]
    fn len(&self) -> usize {
        self.iter_a.len() + self.iter_b.len()
    }
}
