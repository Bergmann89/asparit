use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::panicking;

use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithIndexedProducer,
    WithProducer, WithSetup,
};

/* PanicFuse */

pub struct PanicFuse<X> {
    base: X,
}

impl<X> PanicFuse<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X> ParallelIterator<'a> for PanicFuse<X>
where
    X: ParallelIterator<'a>,
{
    type Item = X::Item;

    fn drive<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = PanicFuseConsumer {
            base,
            fuse: Fuse::default(),
        };

        self.base.drive(executor, consumer)
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X> IndexedParallelIterator<'a> for PanicFuse<X>
where
    X: IndexedParallelIterator<'a>,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = PanicFuseConsumer {
            base,
            fuse: Fuse::default(),
        };

        self.base.drive_indexed(executor, consumer)
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

impl<'a, X> WithProducer<'a> for PanicFuse<X>
where
    X: WithProducer<'a>,
{
    type Item = X::Item;

    fn with_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(PanicFuseCallback { base })
    }
}

impl<'a, X> WithIndexedProducer<'a> for PanicFuse<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = X::Item;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(PanicFuseCallback { base })
    }
}

/* Fuse */

struct Fuse(Arc<AtomicBool>);

impl Fuse {
    #[inline]
    fn panicked(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for Fuse {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

impl Clone for Fuse {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Drop for Fuse {
    fn drop(&mut self) {
        if panicking() {
            self.0.store(true, Ordering::Relaxed);
        }
    }
}

/* PanicFuseCallback */

struct PanicFuseCallback<CB> {
    base: CB,
}

impl<'a, CB, T> ProducerCallback<'a, T> for PanicFuseCallback<CB>
where
    CB: ProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(PanicFuseProducer {
            base,
            fuse: Fuse::default(),
        })
    }
}

impl<'a, CB, T> IndexedProducerCallback<'a, T> for PanicFuseCallback<CB>
where
    CB: IndexedProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = T> + 'a,
    {
        self.base.callback(PanicFuseProducer {
            base,
            fuse: Fuse::default(),
        })
    }
}

/* PanicFuseProducer */

struct PanicFuseProducer<P> {
    base: P,
    fuse: Fuse,
}

impl<P> WithSetup for PanicFuseProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<P> Producer for PanicFuseProducer<P>
where
    P: Producer,
{
    type Item = P::Item;
    type IntoIter = PanicFuseIter<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        PanicFuseIter {
            base: self.base.into_iter(),
            fuse: self.fuse,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let fuse = self.fuse;
        let (left, right) = self.base.split();

        let left = Self {
            base: left,
            fuse: fuse.clone(),
        };
        let right = right.map(|base| Self { base, fuse });

        (left, right)
    }

    fn fold_with<F>(self, base: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = PanicFuseFolder {
            base,
            fuse: self.fuse,
        };

        self.base.fold_with(folder).base
    }
}

impl<P> IndexedProducer for PanicFuseProducer<P>
where
    P: IndexedProducer,
{
    type Item = P::Item;
    type IntoIter = PanicFuseIter<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        PanicFuseIter {
            base: self.base.into_iter(),
            fuse: self.fuse,
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = Self {
            base: left,
            fuse: self.fuse.clone(),
        };
        let right = Self {
            base: right,
            fuse: self.fuse,
        };

        (left, right)
    }

    fn fold_with<F>(self, base: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = PanicFuseFolder {
            base,
            fuse: self.fuse.clone(),
        };

        self.base.fold_with(folder).base
    }
}

/* PanicFuseIter */

struct PanicFuseIter<I> {
    base: I,
    fuse: Fuse,
}

impl<I> Iterator for PanicFuseIter<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fuse.panicked() {
            None
        } else {
            self.base.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}

impl<I> DoubleEndedIterator for PanicFuseIter<I>
where
    I: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.fuse.panicked() {
            None
        } else {
            self.base.next_back()
        }
    }
}

impl<I> ExactSizeIterator for PanicFuseIter<I> where I: ExactSizeIterator {}

/* PanicFuseConsumer */

struct PanicFuseConsumer<C> {
    base: C,
    fuse: Fuse,
}

impl<C> WithSetup for PanicFuseConsumer<C>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<C, T> Consumer<T> for PanicFuseConsumer<C>
where
    C: Consumer<T>,
{
    type Folder = PanicFuseFolder<C::Folder>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = Self {
            base: left,
            fuse: self.fuse.clone(),
        };
        let right = Self {
            base: right,
            fuse: self.fuse,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = Self {
            base: left,
            fuse: self.fuse.clone(),
        };
        let right = Self {
            base: right,
            fuse: self.fuse,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        PanicFuseFolder {
            base: self.base.into_folder(),
            fuse: self.fuse,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* PanicFuseFolder */

struct PanicFuseFolder<F> {
    base: F,
    fuse: Fuse,
}

impl<F, T> Folder<T> for PanicFuseFolder<F>
where
    F: Folder<T>,
{
    type Result = F::Result;

    fn consume(mut self, item: T) -> Self {
        self.base = self.base.consume(item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        self.base = self.base.consume_iter(iter);

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full() || self.fuse.panicked()
    }
}
