use crate::{
    Consumer, Executor, ExecutorCallback, Folder, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Producer, ProducerCallback, Reducer,
};

pub struct Splits<X> {
    base: X,
    splits: usize,
}

impl<X> Splits<X> {
    pub fn new(base: X, splits: usize) -> Self {
        Self { base, splits }
    }
}

impl<'a, X> ParallelIterator<'a> for Splits<X>
where
    X: ParallelIterator<'a>,
{
    type Item = X::Item;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_producer(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        let splits = self.splits;

        self.base.with_producer(SplitsCallback { base, splits })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X> IndexedParallelIterator<'a> for Splits<X>
where
    X: IndexedParallelIterator<'a>,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer_indexed<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        let splits = self.splits;

        self.base
            .with_producer_indexed(SplitsCallback { base, splits })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* SplitsCallback */

struct SplitsCallback<CB> {
    base: CB,
    splits: usize,
}

impl<'a, CB, I> ProducerCallback<'a, I> for SplitsCallback<CB>
where
    CB: ProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a,
    {
        let splits = self.splits;

        self.base.callback(SplitsProducer { base, splits })
    }
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for SplitsCallback<CB>
where
    CB: IndexedProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let splits = self.splits;

        self.base.callback(SplitsProducer { base, splits })
    }
}

/* SplitsProducer */

struct SplitsProducer<P> {
    base: P,
    splits: usize,
}

impl<P> Producer for SplitsProducer<P>
where
    P: Producer,
{
    type Item = P::Item;
    type IntoIter = P::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }

    fn split(self) -> (Self, Option<Self>) {
        let splits = self.splits;
        let (left, right) = self.base.split();

        let left = Self { base: left, splits };
        let right = right.map(|base| Self { base, splits });

        (left, right)
    }

    fn splits(&self) -> Option<usize> {
        Some(self.splits)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(folder)
    }
}

impl<P> IndexedProducer for SplitsProducer<P>
where
    P: IndexedProducer,
{
    type Item = P::Item;
    type IntoIter = P::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let splits = self.splits;
        let (left, right) = self.base.split_at(index);

        let left = Self { base: left, splits };
        let right = Self {
            base: right,
            splits,
        };

        (left, right)
    }

    fn splits(&self) -> Option<usize> {
        Some(self.splits)
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
        self.base.fold_with(folder)
    }
}
