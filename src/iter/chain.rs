use std::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};

use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithIndexedProducer,
    WithProducer, WithSetup,
};

/* Chain */

pub struct Chain<X1, X2> {
    iterator_1: X1,
    iterator_2: X2,
}

impl<X1, X2> Chain<X1, X2> {
    pub fn new(iterator_1: X1, iterator_2: X2) -> Self {
        Self {
            iterator_1,
            iterator_2,
        }
    }
}

impl<'a, X1, X2, T> ParallelIterator<'a> for Chain<X1, X2>
where
    X1: ParallelIterator<'a, Item = T>,
    X2: ParallelIterator<'a, Item = T>,
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
        let left = self.iterator_1;
        let right = self.iterator_2;

        let (left_executor, right_executor) = executor.split();

        let (left_consumer, right_consumer, reducer) = match left.len_hint_opt() {
            Some(len) => consumer.split_at(len),
            None => consumer.split(),
        };

        let left = left.drive(left_executor, left_consumer);
        let right = right.drive(right_executor, right_consumer);

        E::join(left, right, reducer)
    }

    fn len_hint_opt(&self) -> Option<usize> {
        let len_1 = self.iterator_1.len_hint_opt();
        let len_2 = self.iterator_2.len_hint_opt();

        match (len_1, len_2) {
            (Some(x1), Some(x2)) => Some(x1 + x2),
            (_, _) => None,
        }
    }
}

impl<'a, X1, X2, T> IndexedParallelIterator<'a> for Chain<X1, X2>
where
    X1: IndexedParallelIterator<'a, Item = T>,
    X2: IndexedParallelIterator<'a, Item = T>,
    T: Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let left = self.iterator_1;
        let right = self.iterator_2;

        let (left_executor, right_executor) = executor.split();
        let (left_consumer, right_consumer, reducer) = consumer.split_at(left.len_hint());

        let left = left.drive(left_executor, left_consumer);
        let right = right.drive(right_executor, right_consumer);

        E::join(left, right, reducer)
    }

    fn len_hint(&self) -> usize {
        self.iterator_1.len_hint() + self.iterator_2.len_hint()
    }
}

impl<'a, X1, X2, T> WithProducer<'a> for Chain<X1, X2>
where
    X1: ParallelIterator<'a, Item = T> + WithProducer<'a, Item = T>,
    X2: ParallelIterator<'a, Item = T> + WithProducer<'a, Item = T>,
    T: Send + 'a,
{
    type Item = T;

    fn with_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.iterator_1.with_producer(ChainCallback1 {
            base,
            iterator_2: self.iterator_2,
        })
    }
}

impl<'a, X1, X2, T> WithIndexedProducer<'a> for Chain<X1, X2>
where
    X1: IndexedParallelIterator<'a, Item = T> + WithIndexedProducer<'a, Item = T>,
    X2: IndexedParallelIterator<'a, Item = T> + WithIndexedProducer<'a, Item = T>,
    T: Send + 'a,
{
    type Item = T;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.iterator_1.with_indexed_producer(ChainCallback1 {
            base,
            iterator_2: self.iterator_2,
        })
    }
}

/* ChainCallback1 */

struct ChainCallback1<CB, X2> {
    base: CB,
    iterator_2: X2,
}

impl<'a, CB, X2, T> ProducerCallback<'a, T> for ChainCallback1<CB, X2>
where
    CB: ProducerCallback<'a, T>,
    X2: ParallelIterator<'a, Item = T> + WithProducer<'a, Item = T>,
    T: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_1: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        let base = self.base;

        self.iterator_2
            .with_producer(ChainCallback2 { base, producer_1 })
    }
}

impl<'a, CB, X2, T> IndexedProducerCallback<'a, T> for ChainCallback1<CB, X2>
where
    CB: IndexedProducerCallback<'a, T>,
    X2: IndexedParallelIterator<'a, Item = T> + WithIndexedProducer<'a, Item = T>,
    T: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_1: P) -> Self::Output
    where
        P: IndexedProducer<Item = T> + 'a,
    {
        let base = self.base;

        self.iterator_2
            .with_indexed_producer(ChainCallback2 { base, producer_1 })
    }
}

/* ChainCallback2 */

struct ChainCallback2<CB, P1> {
    base: CB,
    producer_1: P1,
}

impl<'a, CB, P1, T> ProducerCallback<'a, T> for ChainCallback2<CB, P1>
where
    CB: ProducerCallback<'a, T>,
    P1: Producer<Item = T> + 'a,
    T: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_2: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        let producer_1 = self.producer_1;

        self.base.callback(ChainProducer {
            producer_1: Some(producer_1),
            producer_2: Some(producer_2),
        })
    }
}

impl<'a, CB, P1, T> IndexedProducerCallback<'a, T> for ChainCallback2<CB, P1>
where
    CB: IndexedProducerCallback<'a, T>,
    P1: IndexedProducer<Item = T> + 'a,
    T: Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer_2: P) -> Self::Output
    where
        P: IndexedProducer<Item = T> + 'a,
    {
        let producer_1 = self.producer_1;

        self.base.callback(ChainProducer {
            producer_1: Some(producer_1),
            producer_2: Some(producer_2),
        })
    }
}

/* ChainProducer */

struct ChainProducer<P1, P2> {
    producer_1: Option<P1>,
    producer_2: Option<P2>,
}

impl<P1, P2> WithSetup for ChainProducer<P1, P2>
where
    P1: WithSetup,
    P2: WithSetup,
{
    fn setup(&self) -> Setup {
        let mut ret = Setup::default();

        if let Some(p) = &self.producer_1 {
            ret = ret.merge(p.setup());
        }

        if let Some(p) = &self.producer_2 {
            ret = ret.merge(p.setup());
        }

        ret
    }
}

impl<'a, P1, P2, T> Producer for ChainProducer<P1, P2>
where
    P1: Producer<Item = T>,
    P2: Producer<Item = T>,
{
    type Item = T;
    type IntoIter = ChainIter<P1::IntoIter, P2::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        ChainIter::new(
            self.producer_1.map(Producer::into_iter),
            self.producer_2.map(Producer::into_iter),
        )
    }

    fn split(self) -> (Self, Option<Self>) {
        match (self.producer_1, self.producer_2) {
            (Some(p1), Some(p2)) => {
                let left = ChainProducer {
                    producer_1: Some(p1),
                    producer_2: None,
                };
                let right = Some(ChainProducer {
                    producer_1: None,
                    producer_2: Some(p2),
                });

                (left, right)
            }
            (Some(p1), None) => {
                let (left, right) = p1.split();

                let left = ChainProducer {
                    producer_1: Some(left),
                    producer_2: None,
                };
                let right = right.map(|right| ChainProducer {
                    producer_1: Some(right),
                    producer_2: None,
                });

                (left, right)
            }
            (None, Some(p2)) => {
                let (left, right) = p2.split();

                let left = ChainProducer {
                    producer_1: None,
                    producer_2: Some(left),
                };
                let right = right.map(|right| ChainProducer {
                    producer_1: None,
                    producer_2: Some(right),
                });

                (left, right)
            }
            (None, None) => unreachable!(),
        }
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = match self.producer_1 {
            Some(p) => p.fold_with(folder),
            None => folder,
        };

        if folder.is_full() {
            return folder;
        }

        match self.producer_2 {
            Some(p) => p.fold_with(folder),
            None => folder,
        }
    }
}

impl<'a, P1, P2, T> IndexedProducer for ChainProducer<P1, P2>
where
    P1: IndexedProducer<Item = T>,
    P2: IndexedProducer<Item = T>,
{
    type Item = T;
    type IntoIter = ChainIter<P1::IntoIter, P2::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        ChainIter::new(
            self.producer_1.map(IndexedProducer::into_iter),
            self.producer_2.map(IndexedProducer::into_iter),
        )
    }

    fn len(&self) -> usize {
        let mut len = 0;

        if let Some(p) = &self.producer_1 {
            len += p.len();
        }

        if let Some(p) = &self.producer_2 {
            len += p.len();
        }

        len
    }

    fn split_at(self, mut index: usize) -> (Self, Self) {
        match (self.producer_1, self.producer_2) {
            (Some(p1), Some(p2)) if index < p1.len() => {
                let (left, right) = p1.split_at(index);

                let left = ChainProducer {
                    producer_1: Some(left),
                    producer_2: None,
                };
                let right = ChainProducer {
                    producer_1: Some(right),
                    producer_2: Some(p2),
                };

                (left, right)
            }
            (Some(p1), Some(p2)) if index > p1.len() => {
                index -= p1.len();

                let (left, right) = p2.split_at(index);

                let left = ChainProducer {
                    producer_1: Some(p1),
                    producer_2: Some(left),
                };
                let right = ChainProducer {
                    producer_1: None,
                    producer_2: Some(right),
                };

                (left, right)
            }
            (Some(p1), Some(p2)) => {
                let left = ChainProducer {
                    producer_1: Some(p1),
                    producer_2: None,
                };
                let right = ChainProducer {
                    producer_1: None,
                    producer_2: Some(p2),
                };

                (left, right)
            }
            (Some(p1), None) => {
                let (left, right) = p1.split_at(index);

                let left = ChainProducer {
                    producer_1: Some(left),
                    producer_2: None,
                };
                let right = ChainProducer {
                    producer_1: Some(right),
                    producer_2: None,
                };

                (left, right)
            }
            (None, Some(p2)) => {
                let (left, right) = p2.split_at(index);

                let left = ChainProducer {
                    producer_1: None,
                    producer_2: Some(left),
                };
                let right = ChainProducer {
                    producer_1: None,
                    producer_2: Some(right),
                };

                (left, right)
            }
            (None, None) => unreachable!(),
        }
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = match self.producer_1 {
            Some(p) => p.fold_with(folder),
            None => folder,
        };

        if folder.is_full() {
            return folder;
        }

        match self.producer_2 {
            Some(p) => p.fold_with(folder),
            None => folder,
        }
    }
}

/* ChainIter */

enum ChainIter<I1, I2> {
    Empty,
    Iter1(I1),
    Iter2(I2),
    Chain(std::iter::Chain<I1, I2>),
}

impl<I1, I2, T> ChainIter<I1, I2>
where
    I1: Iterator<Item = T>,
    I2: Iterator<Item = T>,
{
    fn new(i1: Option<I1>, i2: Option<I2>) -> Self {
        match (i1, i2) {
            (Some(i1), Some(i2)) => Self::Chain(i1.chain(i2)),
            (Some(i1), None) => Self::Iter1(i1),
            (None, Some(i2)) => Self::Iter2(i2),
            (None, None) => Self::Empty,
        }
    }
}

impl<I1, I2, T> Iterator for ChainIter<I1, I2>
where
    I1: Iterator<Item = T>,
    I2: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Iter1(i) => i.next(),
            Self::Iter2(i) => i.next(),
            Self::Chain(i) => i.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Empty => (0, Some(0)),
            Self::Iter1(i) => i.size_hint(),
            Self::Iter2(i) => i.size_hint(),
            Self::Chain(i) => i.size_hint(),
        }
    }
}

impl<I1, I2, T> DoubleEndedIterator for ChainIter<I1, I2>
where
    I1: DoubleEndedIterator<Item = T>,
    I2: DoubleEndedIterator<Item = T>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Iter1(i) => i.next_back(),
            Self::Iter2(i) => i.next_back(),
            Self::Chain(i) => i.next_back(),
        }
    }
}

impl<I1, I2, T> ExactSizeIterator for ChainIter<I1, I2>
where
    I1: ExactSizeIterator<Item = T>,
    I2: ExactSizeIterator<Item = T>,
{
}
