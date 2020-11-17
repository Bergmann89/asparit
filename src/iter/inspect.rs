use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithSetup,
};

/* Inspect */

pub struct Inspect<X, O> {
    base: X,
    operation: O,
}

impl<X, O> Inspect<X, O> {
    pub fn new(base: X, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<'a, X, O> ParallelIterator<'a> for Inspect<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&X::Item) + Clone + Send + 'a,
{
    type Item = X::Item;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            InspectConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(InspectCallback {
            base: callback,
            operation: self.operation,
        })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, O> IndexedParallelIterator<'a> for Inspect<X, O>
where
    X: IndexedParallelIterator<'a>,
    O: Fn(&X::Item) + Clone + Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive_indexed(
            executor,
            InspectConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer_indexed(InspectCallback {
            base: callback,
            operation: self.operation,
        })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* InspectConsumer */

struct InspectConsumer<C, O> {
    base: C,
    operation: O,
}

impl<C, O> WithSetup for InspectConsumer<C, O>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, C, O, T> Consumer<T> for InspectConsumer<C, O>
where
    C: Consumer<T>,
    O: Fn(&T) + Clone + Send,
{
    type Folder = InspectFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = InspectConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = InspectConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = InspectConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = InspectConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        InspectFolder {
            base: self.base.into_folder(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* InspectFolder */

struct InspectFolder<F, O> {
    base: F,
    operation: O,
}

impl<F, O, T> Folder<T> for InspectFolder<F, O>
where
    F: Folder<T>,
    O: Fn(&T) + Clone,
{
    type Result = F::Result;

    fn consume(mut self, item: T) -> Self {
        (self.operation)(&item);

        self.base = self.base.consume(item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        self.base = self
            .base
            .consume_iter(iter.into_iter().inspect(self.operation.clone()));

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* InspectCallback */

struct InspectCallback<CB, O> {
    base: CB,
    operation: O,
}

impl<'a, CB, O, T> ProducerCallback<'a, T> for InspectCallback<CB, O>
where
    CB: ProducerCallback<'a, T>,
    O: Fn(&T) + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(InspectProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

impl<'a, CB, O, T> IndexedProducerCallback<'a, T> for InspectCallback<CB, O>
where
    CB: IndexedProducerCallback<'a, T>,
    O: Fn(&T) + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = T> + 'a,
    {
        self.base.callback(InspectProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

/* InspectProducer */

struct InspectProducer<P, O> {
    base: P,
    operation: O,
}

impl<P, O> WithSetup for InspectProducer<P, O>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, P, O, T> Producer for InspectProducer<P, O>
where
    P: Producer<Item = T>,
    O: Fn(&T) + Clone + Send,
{
    type Item = T;
    type IntoIter = std::iter::Inspect<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().inspect(self.operation)
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = InspectProducer {
            base: left,
            operation: operation.clone(),
        };
        let right = right.map(move |right| InspectProducer {
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
            .fold_with(InspectFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}

impl<'a, P, O, T> IndexedProducer for InspectProducer<P, O>
where
    P: IndexedProducer<Item = T>,
    O: Fn(&T) + Clone + Send,
{
    type Item = T;
    type IntoIter = std::iter::Inspect<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().inspect(self.operation)
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = InspectProducer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = InspectProducer {
            base: right,
            operation: self.operation,
        };

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base
            .fold_with(InspectFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}
