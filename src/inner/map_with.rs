use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer,
};

/* MapWith */

pub struct MapWith<X, S, O> {
    base: X,
    item: S,
    operation: O,
}

impl<X, S, O> MapWith<X, S, O> {
    pub fn new(base: X, item: S, operation: O) -> Self {
        Self {
            base,
            item,
            operation,
        }
    }
}

impl<'a, X, O, T, S> ParallelIterator<'a> for MapWith<X, S, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&mut S, X::Item) -> T + Clone + Sync + Send + 'a,
    T: Send + 'a,
    S: Clone + Send + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = MapWithConsumer::new(consumer, self.item, self.operation);

        self.base.drive(executor, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(MapWithCallback {
            callback,
            item: self.item,
            operation: self.operation,
        })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, O, T, S> IndexedParallelIterator<'a> for MapWith<X, S, O>
where
    X: IndexedParallelIterator<'a>,
    O: Fn(&mut S, X::Item) -> T + Clone + Sync + Send + 'a,
    T: Send + 'a,
    S: Clone + Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let consumer = MapWithConsumer::new(consumer, self.item, self.operation);

        self.base.drive_indexed(executor, consumer)
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer_indexed(MapWithCallback {
            callback,
            item: self.item,
            operation: self.operation,
        })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* MapWithCallback */

struct MapWithCallback<CB, S, O> {
    callback: CB,
    item: S,
    operation: O,
}

impl<'a, I, S, O, T, CB> ProducerCallback<'a, I> for MapWithCallback<CB, S, O>
where
    CB: ProducerCallback<'a, T>,
    O: Fn(&mut S, I) -> T + Clone + Sync + Send + 'a,
    T: Send,
    S: Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: Producer<Item = I> + 'a,
    {
        let producer = MapWithProducer {
            base,
            item: self.item,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

impl<'a, I, S, O, T, CB> IndexedProducerCallback<'a, I> for MapWithCallback<CB, S, O>
where
    CB: IndexedProducerCallback<'a, T>,
    O: Fn(&mut S, I) -> T + Clone + Sync + Send + 'a,
    T: Send,
    S: Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> CB::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let producer = MapWithProducer {
            base,
            item: self.item,
            operation: self.operation,
        };

        self.callback.callback(producer)
    }
}

/* MapWithProducer */

struct MapWithProducer<P, S, O> {
    base: P,
    item: S,
    operation: O,
}

impl<P, S, O, T> Producer for MapWithProducer<P, S, O>
where
    P: Producer,
    O: Fn(&mut S, P::Item) -> T + Clone + Sync + Send,
    T: Send,
    S: Clone + Send,
{
    type Item = T;
    type IntoIter = MapWithIter<P::IntoIter, S, O>;

    fn into_iter(self) -> Self::IntoIter {
        MapWithIter {
            base: self.base.into_iter(),
            item: self.item,
            operation: self.operation,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let item = self.item;
        let operation = self.operation;
        let (left, right) = self.base.split();

        (
            MapWithProducer {
                base: left,
                item: item.clone(),
                operation: operation.clone(),
            },
            right.map(|right| MapWithProducer {
                base: right,
                item,
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
            item: self.item,
            operation: self.operation,
        };

        self.base.fold_with(folder).base
    }
}

impl<P, S, O, T> IndexedProducer for MapWithProducer<P, S, O>
where
    P: IndexedProducer,
    O: Fn(&mut S, P::Item) -> T + Clone + Sync + Send,
    T: Send,
    S: Clone + Send,
{
    type Item = T;
    type IntoIter = MapWithIter<P::IntoIter, S, O>;

    fn into_iter(self) -> Self::IntoIter {
        MapWithIter {
            base: self.base.into_iter(),
            item: self.item,
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
            MapWithProducer {
                base: left,
                item: self.item.clone(),
                operation: self.operation.clone(),
            },
            MapWithProducer {
                base: right,
                item: self.item,
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
            item: self.item,
            operation: self.operation,
        };

        self.base.fold_with(folder).base
    }
}

/* MapWithIter */

pub struct MapWithIter<I, S, O> {
    pub base: I,
    pub item: S,
    pub operation: O,
}

impl<I, S, O, T> Iterator for MapWithIter<I, S, O>
where
    I: Iterator,
    O: Fn(&mut S, I::Item) -> T,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let item = self.base.next()?;

        Some((self.operation)(&mut self.item, item))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }
}

impl<I, S, O, T> DoubleEndedIterator for MapWithIter<I, S, O>
where
    I: DoubleEndedIterator,
    O: Fn(&mut S, I::Item) -> T,
{
    fn next_back(&mut self) -> Option<T> {
        let item = self.base.next_back()?;

        Some((self.operation)(&mut self.item, item))
    }
}

impl<I, S, O, T> ExactSizeIterator for MapWithIter<I, S, O>
where
    I: ExactSizeIterator,
    O: Fn(&mut S, I::Item) -> T,
{
}

/* MapWithConsumer */

struct MapWithConsumer<C, S, O> {
    base: C,
    item: S,
    operation: O,
}

impl<C, S, O> MapWithConsumer<C, S, O> {
    fn new(base: C, item: S, operation: O) -> Self {
        Self {
            base,
            item,
            operation,
        }
    }
}

impl<I, T, C, S, O> Consumer<I> for MapWithConsumer<C, S, O>
where
    C: Consumer<T>,
    O: Fn(&mut S, I) -> T + Clone + Send + Sync,
    T: Send,
    S: Clone + Send,
{
    type Folder = MapWithFolder<C::Folder, S, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = MapWithConsumer::new(left, self.item.clone(), self.operation.clone());
        let right = MapWithConsumer::new(right, self.item, self.operation);

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = MapWithConsumer::new(left, self.item.clone(), self.operation.clone());
        let right = MapWithConsumer::new(right, self.item, self.operation);

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        MapWithFolder {
            base: self.base.into_folder(),
            item: self.item,
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* MapWithFolder */

pub struct MapWithFolder<F, S, O> {
    pub base: F,
    pub item: S,
    pub operation: O,
}

impl<I, T, F, S, O> Folder<I> for MapWithFolder<F, S, O>
where
    F: Folder<T>,
    O: Fn(&mut S, I) -> T + Clone,
{
    type Result = F::Result;

    fn consume(mut self, item: I) -> Self {
        let mapped_item = (self.operation)(&mut self.item, item);

        self.base = self.base.consume(mapped_item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        fn with<'f, I, S, T>(
            item: &'f mut S,
            operation: impl Fn(&mut S, I) -> T + 'f,
        ) -> impl FnMut(I) -> T + 'f {
            move |x| operation(item, x)
        }

        let mapped_iter = iter
            .into_iter()
            .map(with(&mut self.item, self.operation.clone()));

        self.base = self.base.consume_iter(mapped_iter);

        self
    }

    fn complete(self) -> F::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
