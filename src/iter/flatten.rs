use std::iter::IntoIterator;

use crate::{
    Consumer, Executor, Folder, ParallelIterator, Producer, ProducerCallback, Reducer, Setup,
    WithSetup,
};

/* FlattenIter */

pub struct FlattenIter<X> {
    base: X,
}

impl<X> FlattenIter<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X, SI> ParallelIterator<'a> for FlattenIter<X>
where
    X: ParallelIterator<'a, Item = SI>,
    SI: IntoIterator + Send + 'a,
    SI::Item: Send + 'a,
{
    type Item = SI::Item;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            FlatMapIterConsumer {
                base: consumer,
                operation: |x| x,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(FlatMapIterCallback {
            base: callback,
            operation: |x| x,
        })
    }
}

/* FlatMapIter */

pub struct FlatMapIter<X, O> {
    base: X,
    operation: O,
}

impl<X, O> FlatMapIter<X, O> {
    pub fn new(base: X, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<'a, X, O, SI> ParallelIterator<'a> for FlatMapIter<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(X::Item) -> SI + Clone + Send + 'a,
    SI: IntoIterator + 'a,
    SI::Item: Send + 'a,
{
    type Item = SI::Item;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            FlatMapIterConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(FlatMapIterCallback {
            base: callback,
            operation: self.operation,
        })
    }
}

/* FlatMapIterConsumer */

struct FlatMapIterConsumer<C, O> {
    base: C,
    operation: O,
}

impl<C, O> WithSetup for FlatMapIterConsumer<C, O>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, C, O, T, SI> Consumer<T> for FlatMapIterConsumer<C, O>
where
    C: Consumer<SI::Item>,
    O: Fn(T) -> SI + Clone + Send,
    SI: IntoIterator,
{
    type Folder = FlatMapIterFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = FlatMapIterConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = FlatMapIterConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = FlatMapIterConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = FlatMapIterConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        FlatMapIterFolder {
            base: self.base.into_folder(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FlatMapIterFolder */

struct FlatMapIterFolder<F, O> {
    base: F,
    operation: O,
}

impl<F, O, T, SI> Folder<T> for FlatMapIterFolder<F, O>
where
    F: Folder<SI::Item>,
    O: Fn(T) -> SI + Clone,
    SI: IntoIterator,
{
    type Result = F::Result;

    fn consume(mut self, item: T) -> Self {
        let iter = (self.operation)(item);

        self.base = self.base.consume_iter(iter);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        self.base = self
            .base
            .consume_iter(iter.into_iter().flat_map(self.operation.clone()));

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FlatMapIterCallback */

struct FlatMapIterCallback<CB, O> {
    base: CB,
    operation: O,
}

impl<'a, CB, O, T, SI> ProducerCallback<'a, T> for FlatMapIterCallback<CB, O>
where
    CB: ProducerCallback<'a, SI::Item>,
    O: Fn(T) -> SI + Clone + Send + 'a,
    SI: IntoIterator,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(FlatMapIterProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

/* FlatMapIterProducer */

struct FlatMapIterProducer<P, O> {
    base: P,
    operation: O,
}

impl<P, O> WithSetup for FlatMapIterProducer<P, O>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, P, O, T, SI> Producer for FlatMapIterProducer<P, O>
where
    P: Producer<Item = T>,
    O: Fn(T) -> SI + Clone + Send,
    SI: IntoIterator,
{
    type Item = SI::Item;
    type IntoIter = std::iter::FlatMap<P::IntoIter, SI, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().flat_map(self.operation)
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = FlatMapIterProducer {
            base: left,
            operation: operation.clone(),
        };
        let right = right.map(move |right| FlatMapIterProducer {
            base: right,
            operation,
        });

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base
            .fold_with(FlatMapIterFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}
