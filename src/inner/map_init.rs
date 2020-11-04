use crate::{
    Consumer, Folder, IndexedConsumer, IndexedParallelIterator, IndexedProducer, Reducer,
    IndexedProducerCallback, ParallelIterator, Producer, ProducerCallback, Executor,
};

use super::map_with::{MapWithIter, MapWithFolder};

/* MapInit */

pub struct MapInit<X, S, O> {
    base: X,
    init: S,
    operation: O,
}

impl<X, S, O> MapInit<X, S, O> {
    pub fn new(base: X, init: S, operation: O) -> Self {
        Self { base, init, operation }
    }
}

impl<'a, X, O, T, S, U> ParallelIterator<'a> for MapInit<X, S, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&mut U, X::Item) -> T + Clone + Sync + Send + 'a,
    T: Send,
    S: Fn() -> U + Clone + Send + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send,
        R: Reducer<D> + Send,
    {
        let consumer = MapInitConsumer::new(consumer, self.init, self.operation);

        self.base.drive(executor, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(MapInitCallback {
            callback,
            init: self.init,
            operation: self.operation,
        })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, O, T, S, U> IndexedParallelIterator<'a> for MapInit<X, S, O>
where
    X: IndexedParallelIterator<'a>,
    O: Fn(&mut U, X::Item) -> T + Clone + Sync + Send + 'a,
    T: Send,
    S: Fn() -> U + Clone + Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: IndexedConsumer<Self::Item, Result = D, Reducer = R>,
        D: Send,
        R: Reducer<D>
    {
        let consumer = MapInitConsumer::new(consumer, self.init, self.operation);

        self.base.drive_indexed(executor, consumer)
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer_indexed(MapInitCallback {
            callback,
            init: self.init,
            operation: self.operation,
        })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* MapInitCallback */

struct MapInitCallback<CB, S, O> {
    callback: CB,
    init: S,
    operation: O,
}

impl<'a, I, S, O, T, U, CB> ProducerCallback<'a, I> for MapInitCallback<CB, S, O>
where
    CB: ProducerCallback<'a, T>,
    O: Fn(&mut U, I) -> T + Clone + Sync + Send + 'a,
    T: Send,
    S: Fn() -> U + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: Producer<Item = I> + 'a,
    {
        let producer = MapInitProducer {
            base,
            init: self.init,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

impl<'a, I, S, O, T, U, CB> IndexedProducerCallback<'a, I> for MapInitCallback<CB, S, O>
where
    CB: IndexedProducerCallback<'a, T>,
    O: Fn(&mut U, I) -> T + Clone + Sync + Send + 'a,
    T: Send,
    S: Fn() -> U + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let producer = MapInitProducer {
            base,
            init: self.init,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

/* MapInitProducer */

struct MapInitProducer<P, S, O> {
    base: P,
    init: S,
    operation: O,
}

impl<P, S, O, T, U> Producer for MapInitProducer<P, S, O>
where
    P: Producer,
    O: Fn(&mut U, P::Item) -> T + Clone + Sync + Send,
    T: Send,
    S: Fn() -> U + Clone + Send,
{
    type Item = T;
    type IntoIter = MapWithIter<P::IntoIter, U, O>;

    fn into_iter(self) -> Self::IntoIter {
        MapWithIter {
            base: self.base.into_iter(),
            item: (self.init)(),
            operation: self.operation,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let init = self.init;
        let operation = self.operation;
        let (left, right) = self.base.split();

        (
            MapInitProducer {
                base: left,
                init: init.clone(),
                operation: operation.clone(),
            },
            right.map(|right| MapInitProducer {
                base: right,
                init,
                operation,
            }),
        )
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = MapWithFolder {
            base: folder,
            item: (self.init)(),
            operation: self.operation,
        };

        self.base.fold_with(folder).base
    }
}

impl<P, S, O, T, U> IndexedProducer for MapInitProducer<P, S, O>
where
    P: IndexedProducer,
    O: Fn(&mut U, P::Item) -> T + Clone + Sync + Send,
    T: Send,
    S: Fn() -> U + Clone + Send,
{
    type Item = T;
    type IntoIter = MapWithIter<P::IntoIter, U, O>;

    fn into_iter(self) -> Self::IntoIter {
        MapWithIter {
            base: self.base.into_iter(),
            item: (self.init)(),
            operation: self.operation,
        }
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
            MapInitProducer {
                base: left,
                init: self.init.clone(),
                operation: self.operation.clone(),
            },
            MapInitProducer {
                base: right,
                init: self.init,
                operation: self.operation,
            },
        )
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let folder = MapWithFolder {
            base: folder,
            item: (self.init)(),
            operation: self.operation,
        };

        self.base.fold_with(folder).base
    }
}

/* MapInitConsumer */

struct MapInitConsumer<C, S, O> {
    base: C,
    init: S,
    operation: O,
}

impl<C, S, O> MapInitConsumer<C, S, O> {
    fn new(base: C, init: S, operation: O) -> Self {
        Self { base, init, operation }
    }
}

impl<I, T, C, S, U, O> Consumer<I> for MapInitConsumer<C, S, O>
where
    C: Consumer<T>,
    O: Fn(&mut U, I) -> T + Clone + Send + Sync,
    T: Send,
    S: Fn() -> U + Clone + Send,
{
    type Folder = MapWithFolder<C::Folder, U, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split_off_left(&self) -> (Self, Self::Reducer) {
        let (left, reducer) = self.base.split_off_left();

        (MapInitConsumer::new(left, self.init.clone(), self.operation.clone()), reducer)
    }

    fn into_folder(self) -> Self::Folder {
        MapWithFolder {
            base: self.base.into_folder(),
            item: (self.init)(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

impl<I, T, C, S, U, O> IndexedConsumer<I> for MapInitConsumer<C, S, O>
where
    C: IndexedConsumer<T>,
    O: Fn(&mut U, I) -> T + Clone + Send + Sync,
    T: Send,
    S: Fn() -> U + Clone + Send,
{
    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        (
            MapInitConsumer::new(left, self.init.clone(), self.operation.clone()),
            MapInitConsumer::new(right, self.init, self.operation),
            reducer,
        )
    }
}
