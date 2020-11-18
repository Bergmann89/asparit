use std::iter::{DoubleEndedIterator, ExactSizeIterator, Iterator};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithIndexedProducer,
    WithProducer, WithSetup,
};

/* WhileSome */

pub struct WhileSome<X> {
    base: X,
}

impl<X> WhileSome<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X, T> ParallelIterator<'a> for WhileSome<X>
where
    X: ParallelIterator<'a, Item = Option<T>>,
    T: Send + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = WhileSomeConsumer {
            base,
            is_full: Arc::new(AtomicBool::new(false)),
        };

        self.base.drive(executor, consumer)
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, T> IndexedParallelIterator<'a> for WhileSome<X>
where
    X: IndexedParallelIterator<'a, Item = Option<T>>,
    T: Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = WhileSomeConsumer {
            base,
            is_full: Arc::new(AtomicBool::new(false)),
        };

        self.base.drive_indexed(executor, consumer)
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

impl<'a, X, T> WithProducer<'a> for WhileSome<X>
where
    X: WithProducer<'a, Item = Option<T>>,
    T: Send + 'a,
{
    type Item = T;

    fn with_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(WhileSomeCallback { base })
    }
}

impl<'a, X, T> WithIndexedProducer<'a> for WhileSome<X>
where
    X: WithIndexedProducer<'a, Item = Option<T>>,
    T: Send + 'a,
{
    type Item = T;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(WhileSomeCallback { base })
    }
}

/* WhileSomeCallback */

struct WhileSomeCallback<CB> {
    base: CB,
}

impl<'a, CB, T> ProducerCallback<'a, Option<T>> for WhileSomeCallback<CB>
where
    CB: ProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: Producer<Item = Option<T>> + 'a,
    {
        self.base.callback(WhileSomeProducer {
            base,
            is_full: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl<'a, CB, T> IndexedProducerCallback<'a, Option<T>> for WhileSomeCallback<CB>
where
    CB: IndexedProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = Option<T>> + 'a,
    {
        self.base.callback(WhileSomeProducer {
            base,
            is_full: Arc::new(AtomicBool::new(false)),
        })
    }
}

/* WhileSomeProducer */

struct WhileSomeProducer<P> {
    base: P,
    is_full: Arc<AtomicBool>,
}

impl<P> WithSetup for WhileSomeProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<P, T> Producer for WhileSomeProducer<P>
where
    P: Producer<Item = Option<T>>,
{
    type Item = T;
    type IntoIter = WhileSomeIter<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        WhileSomeIter {
            base: self.base.into_iter(),
            is_full: self.is_full,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let is_full = self.is_full;
        let (left, right) = self.base.split();

        let left = Self {
            base: left,
            is_full: is_full.clone(),
        };
        let right = right.map(|x| Self { base: x, is_full });

        (left, right)
    }
}

impl<P, T> IndexedProducer for WhileSomeProducer<P>
where
    P: IndexedProducer<Item = Option<T>>,
{
    type Item = T;
    type IntoIter = WhileSomeIter<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        WhileSomeIter {
            base: self.base.into_iter(),
            is_full: self.is_full,
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = Self {
            base: left,
            is_full: self.is_full.clone(),
        };
        let right = Self {
            base: right,
            is_full: self.is_full,
        };

        (left, right)
    }
}

/* WhileSomeIter */

struct WhileSomeIter<I> {
    base: I,
    is_full: Arc<AtomicBool>,
}

impl<I, T> Iterator for WhileSomeIter<I>
where
    I: Iterator<Item = Option<T>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_full.load(Ordering::Relaxed) {
            return None;
        }

        match self.base.next()? {
            Some(next) => Some(next),
            None => {
                self.is_full.store(true, Ordering::Relaxed);

                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.is_full.load(Ordering::Relaxed) {
            (0, Some(0))
        } else {
            let (_min, max) = self.base.size_hint();

            (0, max)
        }
    }
}

impl<I, T> DoubleEndedIterator for WhileSomeIter<I>
where
    I: DoubleEndedIterator<Item = Option<T>>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.is_full.load(Ordering::Relaxed) {
            return None;
        }

        match self.base.next()? {
            Some(next) => Some(next),
            None => {
                self.is_full.store(true, Ordering::Relaxed);

                None
            }
        }
    }
}

impl<I, T> ExactSizeIterator for WhileSomeIter<I> where I: ExactSizeIterator<Item = Option<T>> {}

/* WhileSomeConsumer */

struct WhileSomeConsumer<C> {
    base: C,
    is_full: Arc<AtomicBool>,
}

impl<C> WithSetup for WhileSomeConsumer<C>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<C, T> Consumer<Option<T>> for WhileSomeConsumer<C>
where
    C: Consumer<T>,
    T: Send,
{
    type Folder = WhileSomeFolder<C::Folder>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = Self {
            base: left,
            is_full: self.is_full.clone(),
        };
        let right = Self {
            base: right,
            is_full: self.is_full,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = Self {
            base: left,
            is_full: self.is_full.clone(),
        };
        let right = Self {
            base: right,
            is_full: self.is_full,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        let base = self.base.into_folder();

        WhileSomeFolder {
            base,
            is_full: self.is_full,
        }
    }

    fn is_full(&self) -> bool {
        self.is_full.load(Ordering::Relaxed) || self.base.is_full()
    }
}

/* WhileSomeFolder */

struct WhileSomeFolder<F> {
    base: F,
    is_full: Arc<AtomicBool>,
}

impl<F, I> Folder<Option<I>> for WhileSomeFolder<F>
where
    F: Folder<I>,
{
    type Result = F::Result;

    fn consume(mut self, item: Option<I>) -> Self {
        match item {
            Some(item) => self.base = self.base.consume(item),
            None => self.is_full.store(true, Ordering::Relaxed),
        }

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = Option<I>>,
    {
        let is_full = self.is_full.clone();

        let iter = iter
            .into_iter()
            .take_while(|x| match *x {
                Some(_) => !is_full.load(Ordering::Relaxed),
                None => {
                    is_full.store(true, Ordering::Relaxed);

                    false
                }
            })
            .map(Option::unwrap);

        self.base = self.base.consume_iter(iter);

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.is_full.load(Ordering::Relaxed) || self.base.is_full()
    }
}
