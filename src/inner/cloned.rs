use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer,
};

/* Cloned */

pub struct Cloned<X> {
    base: X,
}

impl<X> Cloned<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X, T> ParallelIterator<'a> for Cloned<X>
where
    X: ParallelIterator<'a, Item = &'a T>,
    T: Clone + Send + Sync + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(executor, ClonedConsumer { base: consumer })
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(ClonedCallback { base: callback })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, T> IndexedParallelIterator<'a> for Cloned<X>
where
    X: IndexedParallelIterator<'a, Item = &'a T>,
    T: Clone + Send + Sync + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base
            .drive_indexed(executor, ClonedConsumer { base: consumer })
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base
            .with_producer_indexed(ClonedCallback { base: callback })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* ClonedConsumer */

struct ClonedConsumer<C> {
    base: C,
}

impl<'a, T, C> Consumer<&'a T> for ClonedConsumer<C>
where
    T: Clone,
    C: Consumer<T>,
{
    type Folder = ClonedFolder<C::Folder>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = ClonedConsumer { base: left };
        let right = ClonedConsumer { base: right };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = ClonedConsumer { base: left };
        let right = ClonedConsumer { base: right };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        ClonedFolder {
            base: self.base.into_folder(),
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* ClonedFolder */

struct ClonedFolder<F> {
    base: F,
}

impl<'a, T, F> Folder<&'a T> for ClonedFolder<F>
where
    T: Clone + 'a,
    F: Folder<T>,
{
    type Result = F::Result;

    fn consume(mut self, item: &'a T) -> Self {
        self.base = self.base.consume(item.clone());

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = &'a T>,
    {
        self.base = self.base.consume_iter(iter.into_iter().cloned());

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* ClonedCallback */

struct ClonedCallback<CB> {
    base: CB,
}

impl<'a, T, CB> ProducerCallback<'a, &'a T> for ClonedCallback<CB>
where
    T: Clone + 'a,
    CB: ProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = &'a T> + 'a,
    {
        self.base.callback(ClonedProducer { base: producer })
    }
}

impl<'a, T, CB> IndexedProducerCallback<'a, &'a T> for ClonedCallback<CB>
where
    T: Clone + 'a,
    CB: IndexedProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = &'a T> + 'a,
    {
        self.base.callback(ClonedProducer { base: producer })
    }
}

/* ClonedProducer */

struct ClonedProducer<P> {
    base: P,
}

impl<'a, T, P> Producer for ClonedProducer<P>
where
    T: Clone + 'a,
    P: Producer<Item = &'a T>,
{
    type Item = T;
    type IntoIter = std::iter::Cloned<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().cloned()
    }

    fn split(self) -> (Self, Option<Self>) {
        let (left, right) = self.base.split();

        let left = ClonedProducer { base: left };
        let right = right.map(|right| ClonedProducer { base: right });

        (left, right)
    }

    fn splits(&self) -> Option<usize> {
        self.base.splits()
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(ClonedFolder { base: folder }).base
    }
}

impl<'a, T, P> IndexedProducer for ClonedProducer<P>
where
    T: Clone + 'a,
    P: IndexedProducer<Item = &'a T>,
{
    type Item = T;
    type IntoIter = std::iter::Cloned<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().cloned()
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = ClonedProducer { base: left };
        let right = ClonedProducer { base: right };

        (left, right)
    }

    fn splits(&self) -> Option<usize> {
        self.base.splits()
    }

    fn min_len(&self) -> Option<usize> {
        self.base.min_len()
    }

    fn max_len(&self) -> Option<usize> {
        self.base.max_len()
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(ClonedFolder { base: folder }).base
    }
}
