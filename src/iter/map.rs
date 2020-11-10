use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer,
};

/* Map */

pub struct Map<X, O> {
    base: X,
    operation: O,
}

impl<X, O> Map<X, O> {
    pub fn new(base: X, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<'a, X, O, T> ParallelIterator<'a> for Map<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(X::Item) -> T + Clone + Send + 'a,
    T: Send + 'a,
{
    type Item = O::Output;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = MapConsumer::new(consumer, self.operation);

        self.base.drive(executor, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(MapCallback {
            callback,
            operation: self.operation,
        })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, O, T> IndexedParallelIterator<'a> for Map<X, O>
where
    X: IndexedParallelIterator<'a>,
    O: Fn(X::Item) -> T + Clone + Send + 'a,
    T: Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = MapConsumer::new(consumer, self.operation);

        self.base.drive_indexed(executor, consumer)
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer_indexed(MapCallback {
            callback,
            operation: self.operation,
        })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* MapCallback */

struct MapCallback<CB, O> {
    callback: CB,
    operation: O,
}

impl<'a, I, O, T, CB> ProducerCallback<'a, I> for MapCallback<CB, O>
where
    CB: ProducerCallback<'a, T>,
    O: Fn(I) -> T + Clone + Send + 'a,
    T: Send,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: Producer<Item = I> + 'a,
    {
        let producer = MapProducer {
            base,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

impl<'a, I, O, T, CB> IndexedProducerCallback<'a, I> for MapCallback<CB, O>
where
    CB: IndexedProducerCallback<'a, T>,
    O: Fn(I) -> T + Clone + Send + 'a,
    T: Send,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let producer = MapProducer {
            base,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

/* MapProducer */

struct MapProducer<P, O> {
    base: P,
    operation: O,
}

impl<P, O, T> Producer for MapProducer<P, O>
where
    P: Producer,
    O: Fn(P::Item) -> T + Clone + Send,
    T: Send,
{
    type Item = O::Output;
    type IntoIter = std::iter::Map<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().map(self.operation)
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        (
            MapProducer {
                base: left,
                operation: operation.clone(),
            },
            right.map(|right| MapProducer {
                base: right,
                operation,
            }),
        )
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = MapFolder {
            base: folder,
            operation: self.operation,
        };

        self.base.fold_with(folder).base
    }
}

impl<P, O, T> IndexedProducer for MapProducer<P, O>
where
    P: IndexedProducer,
    O: Fn(P::Item) -> T + Clone + Send,
    T: Send,
{
    type Item = O::Output;
    type IntoIter = std::iter::Map<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().map(self.operation)
    }

    fn splits(&self) -> Option<usize> {
        self.base.splits()
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn min_len(&self) -> Option<usize> {
        self.base.min_len()
    }

    fn max_len(&self) -> Option<usize> {
        self.base.max_len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        (
            MapProducer {
                base: left,
                operation: self.operation.clone(),
            },
            MapProducer {
                base: right,
                operation: self.operation,
            },
        )
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = MapFolder {
            base: folder,
            operation: self.operation,
        };

        self.base.fold_with(folder).base
    }
}

/* MapConsumer */

struct MapConsumer<C, O> {
    base: C,
    operation: O,
}

impl<C, O> MapConsumer<C, O> {
    fn new(base: C, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<I, T, C, O> Consumer<I> for MapConsumer<C, O>
where
    C: Consumer<O::Output>,
    O: Fn(I) -> T + Clone + Send,
    T: Send,
{
    type Folder = MapFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = MapConsumer::new(left, self.operation.clone());
        let right = MapConsumer::new(right, self.operation);

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = MapConsumer::new(left, self.operation.clone());
        let right = MapConsumer::new(right, self.operation);

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        MapFolder {
            base: self.base.into_folder(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* MapFolder */

struct MapFolder<F, O> {
    base: F,
    operation: O,
}

impl<I, T, F, O> Folder<I> for MapFolder<F, O>
where
    F: Folder<O::Output>,
    O: Fn(I) -> T + Clone,
{
    type Result = F::Result;

    fn consume(mut self, item: I) -> Self {
        let mapped_item = (self.operation)(item);

        self.base = self.base.consume(mapped_item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        self.base = self
            .base
            .consume_iter(iter.into_iter().map(self.operation.clone()));

        self
    }

    fn complete(self) -> F::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
