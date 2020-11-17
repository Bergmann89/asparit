use crate::{
    Consumer, Executor, Folder, ParallelIterator, Producer, ProducerCallback, Reducer, Setup,
    WithSetup,
};

/* FilterMap */

pub struct FilterMap<X, O> {
    base: X,
    operation: O,
}

impl<X, O> FilterMap<X, O> {
    pub fn new(base: X, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<'a, X, O, S> ParallelIterator<'a> for FilterMap<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(X::Item) -> Option<S> + Clone + Send + 'a,
    S: Send + 'a,
{
    type Item = S;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            FilterMapConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(FilterMapCallback {
            base: callback,
            operation: self.operation,
        })
    }
}

/* FilterMapConsumer */

struct FilterMapConsumer<C, O> {
    base: C,
    operation: O,
}

impl<C, O> WithSetup for FilterMapConsumer<C, O>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, C, O, T, S> Consumer<T> for FilterMapConsumer<C, O>
where
    C: Consumer<S>,
    O: Fn(T) -> Option<S> + Clone + Send,
{
    type Folder = FilterMapFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = FilterMapConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = FilterMapConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = FilterMapConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = FilterMapConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        FilterMapFolder {
            base: self.base.into_folder(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FilterMapFolder */

struct FilterMapFolder<F, O> {
    base: F,
    operation: O,
}

impl<F, O, T, S> Folder<T> for FilterMapFolder<F, O>
where
    F: Folder<S>,
    O: Fn(T) -> Option<S> + Clone,
{
    type Result = F::Result;

    fn consume(mut self, item: T) -> Self {
        if let Some(item) = (self.operation)(item) {
            self.base = self.base.consume(item)
        }

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        self.base = self
            .base
            .consume_iter(iter.into_iter().filter_map(self.operation.clone()));

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FilterMapCallback */

struct FilterMapCallback<CB, O> {
    base: CB,
    operation: O,
}

impl<'a, CB, O, T, S> ProducerCallback<'a, T> for FilterMapCallback<CB, O>
where
    CB: ProducerCallback<'a, S>,
    O: Fn(T) -> Option<S> + Clone + Send + 'a,
    T: Send,
    S: Send,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(FilterMapProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

/* FilterMapProducer */

struct FilterMapProducer<P, O> {
    base: P,
    operation: O,
}

impl<P, O> WithSetup for FilterMapProducer<P, O>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, P, O, T, S> Producer for FilterMapProducer<P, O>
where
    P: Producer<Item = T>,
    O: Fn(T) -> Option<S> + Clone + Send,
    S: Send,
{
    type Item = S;
    type IntoIter = std::iter::FilterMap<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().filter_map(self.operation)
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = FilterMapProducer {
            base: left,
            operation: operation.clone(),
        };
        let right = right.map(move |right| FilterMapProducer {
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
            .fold_with(FilterMapFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}
