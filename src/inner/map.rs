use crate::{
    Consumer, Folder, IndexedConsumer, IndexedParallelIterator, IndexedProducer, Reducer,
    IndexedProducerCallback, ParallelIterator, Producer, ProducerCallback, Executor,
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

impl<X, O, T> ParallelIterator for Map<X, O>
where
    X: ParallelIterator,
    O: Fn(X::Item) -> T + Sync + Send + Copy,
    T: Send,
{
    type Item = O::Output;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<D>,
        C: Consumer<Self::Item, Result = D, Reducer = R>,
        D: Send,
        R: Reducer<D>
    {
        let consumer = MapConsumer::new(consumer, self.operation);

        self.base.drive(executor, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
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

impl<X, O, T> IndexedParallelIterator for Map<X, O>
where
    X: IndexedParallelIterator,
    O: Fn(X::Item) -> T + Sync + Send + Copy,
    T: Send,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<D>,
        C: IndexedConsumer<Self::Item, Result = D, Reducer = R>,
        D: Send,
        R: Reducer<D>
    {
        let consumer = MapConsumer::new(consumer, self.operation);

        self.base.drive_indexed(executor, consumer)
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<Self::Item>,
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

impl<I, O, T, CB> ProducerCallback<I> for MapCallback<CB, O>
where
    CB: ProducerCallback<T>,
    O: Fn(I) -> T + Sync + Send + Copy,
    T: Send,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: Producer<Item = I>,
    {
        let producer = MapProducer {
            base,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

impl<I, O, T, CB> IndexedProducerCallback<I> for MapCallback<CB, O>
where
    CB: IndexedProducerCallback<T>,
    O: Fn(I) -> T + Sync + Send + Copy,
    T: Send,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: IndexedProducer<Item = I>,
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
    O: Fn(P::Item) -> T + Sync + Send + Copy,
    T: Send,
{
    type Item = O::Output;
    type IntoIter = std::iter::Map<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        // self.base.into_iter().map(self.operation)
        unimplemented!()
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        (
            MapProducer {
                base: left,
                operation,
            },
            right.map(|right| MapProducer {
                base: right,
                operation,
            }),
        )
    }

    fn fold_with<G>(self, folder: G) -> G
    where
        G: Folder<Self::Item>,
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
    O: Fn(P::Item) -> T + Sync + Send + Copy,
    T: Send,
{
    type Item = O::Output;
    type IntoIter = std::iter::Map<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().map(self.operation)
    }

    fn min_len(&self) -> usize {
        self.base.min_len()
    }

    fn max_len(&self) -> usize {
        self.base.max_len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let operation = self.operation;
        let (left, right) = self.base.split_at(index);

        (
            MapProducer {
                base: left,
                operation,
            },
            MapProducer {
                base: right,
                operation,
            },
        )
    }

    fn fold_with<G>(self, folder: G) -> G
    where
        G: Folder<Self::Item>,
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
    O: Fn(I) -> T + Send + Sync + Copy,
    T: Send,
{
    type Folder = MapFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_off_left(&self) -> (Self, Self::Reducer) {
        let (left, reducer) = self.base.split_off_left();

        (MapConsumer::new(left, self.operation), reducer)
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

impl<I, T, C, O> IndexedConsumer<I> for MapConsumer<C, O>
where
    C: IndexedConsumer<O::Output>,
    O: Fn(I) -> T + Send + Sync + Copy,
    T: Send,
{
    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        (
            MapConsumer::new(left, self.operation),
            MapConsumer::new(right, self.operation),
            reducer,
        )
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
    O: Fn(I) -> T + Copy,
{
    type Result = F::Result;

    fn consume(self, item: I) -> Self {
        let mapped_item = (self.operation)(item);
        MapFolder {
            base: self.base.consume(mapped_item),
            operation: self.operation,
        }
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        self.base = self.base.consume_iter(iter.into_iter().map(self.operation));

        self
    }

    fn complete(self) -> F::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
