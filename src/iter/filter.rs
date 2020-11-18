use crate::{
    Consumer, Executor, Folder, ParallelIterator, Producer, ProducerCallback, Reducer, Setup,
    WithProducer, WithSetup,
};

/* Filter */

pub struct Filter<X, O> {
    base: X,
    operation: O,
}

impl<X, O> Filter<X, O> {
    pub fn new(base: X, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<'a, X, O> ParallelIterator<'a> for Filter<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&X::Item) -> bool + Clone + Send + 'a,
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
            FilterConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }
}

impl<'a, X, O> WithProducer<'a> for Filter<X, O>
where
    X: WithProducer<'a>,
    O: Fn(&X::Item) -> bool + Clone + Send + 'a,
{
    type Item = X::Item;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(FilterCallback {
            base: callback,
            operation: self.operation,
        })
    }
}

/* FilterConsumer */

struct FilterConsumer<C, O> {
    base: C,
    operation: O,
}

impl<C, O> WithSetup for FilterConsumer<C, O>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, C, O, T> Consumer<T> for FilterConsumer<C, O>
where
    C: Consumer<T>,
    O: Fn(&T) -> bool + Clone + Send,
{
    type Folder = FilterFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = FilterConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = FilterConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = FilterConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = FilterConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        FilterFolder {
            base: self.base.into_folder(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FilterFolder */

struct FilterFolder<F, O> {
    base: F,
    operation: O,
}

impl<F, O, T> Folder<T> for FilterFolder<F, O>
where
    F: Folder<T>,
    O: Fn(&T) -> bool + Clone,
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
            .consume_iter(iter.into_iter().filter(self.operation.clone()));

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FilterCallback */

struct FilterCallback<CB, O> {
    base: CB,
    operation: O,
}

impl<'a, CB, O, T> ProducerCallback<'a, T> for FilterCallback<CB, O>
where
    CB: ProducerCallback<'a, T>,
    O: Fn(&T) -> bool + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(FilterProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

/* FilterProducer */

struct FilterProducer<P, O> {
    base: P,
    operation: O,
}

impl<P, O> WithSetup for FilterProducer<P, O>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, P, O, T> Producer for FilterProducer<P, O>
where
    P: Producer<Item = T>,
    O: Fn(&T) -> bool + Clone + Send,
{
    type Item = T;
    type IntoIter = std::iter::Filter<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().filter(self.operation)
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = FilterProducer {
            base: left,
            operation: operation.clone(),
        };
        let right = right.map(move |right| FilterProducer {
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
            .fold_with(FilterFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}
