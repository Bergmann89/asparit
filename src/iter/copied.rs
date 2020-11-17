use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithSetup,
};

/* Copied */

pub struct Copied<X> {
    base: X,
}

impl<X> Copied<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X, T> ParallelIterator<'a> for Copied<X>
where
    X: ParallelIterator<'a, Item = &'a T>,
    T: Copy + Send + Sync + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(executor, CopiedConsumer { base: consumer })
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(CopiedCallback { base: callback })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, T> IndexedParallelIterator<'a> for Copied<X>
where
    X: IndexedParallelIterator<'a, Item = &'a T>,
    T: Copy + Send + Sync + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base
            .drive_indexed(executor, CopiedConsumer { base: consumer })
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base
            .with_producer_indexed(CopiedCallback { base: callback })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* CopiedConsumer */

struct CopiedConsumer<C> {
    base: C,
}

impl<C> WithSetup for CopiedConsumer<C>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, T, C> Consumer<&'a T> for CopiedConsumer<C>
where
    T: Copy,
    C: Consumer<T>,
{
    type Folder = CopiedFolder<C::Folder>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = CopiedConsumer { base: left };
        let right = CopiedConsumer { base: right };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = CopiedConsumer { base: left };
        let right = CopiedConsumer { base: right };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        CopiedFolder {
            base: self.base.into_folder(),
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* CopiedFolder */

struct CopiedFolder<F> {
    base: F,
}

impl<'a, T, F> Folder<&'a T> for CopiedFolder<F>
where
    T: Copy + 'a,
    F: Folder<T>,
{
    type Result = F::Result;

    fn consume(mut self, item: &'a T) -> Self {
        self.base = self.base.consume(*item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = &'a T>,
    {
        self.base = self.base.consume_iter(iter.into_iter().copied());

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* CopiedCallback */

struct CopiedCallback<CB> {
    base: CB,
}

impl<'a, T, CB> ProducerCallback<'a, &'a T> for CopiedCallback<CB>
where
    T: Copy + 'a,
    CB: ProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = &'a T> + 'a,
    {
        self.base.callback(CopiedProducer { base: producer })
    }
}

impl<'a, T, CB> IndexedProducerCallback<'a, &'a T> for CopiedCallback<CB>
where
    T: Copy + 'a,
    CB: IndexedProducerCallback<'a, T>,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = &'a T> + 'a,
    {
        self.base.callback(CopiedProducer { base: producer })
    }
}

/* CopiedProducer */

struct CopiedProducer<P> {
    base: P,
}

impl<P> WithSetup for CopiedProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, T, P> Producer for CopiedProducer<P>
where
    T: Copy + 'a,
    P: Producer<Item = &'a T>,
{
    type Item = T;
    type IntoIter = std::iter::Copied<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().copied()
    }

    fn split(self) -> (Self, Option<Self>) {
        let (left, right) = self.base.split();

        let left = CopiedProducer { base: left };
        let right = right.map(|right| CopiedProducer { base: right });

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(CopiedFolder { base: folder }).base
    }
}

impl<'a, T, P> IndexedProducer for CopiedProducer<P>
where
    T: Copy + 'a,
    P: IndexedProducer<Item = &'a T>,
{
    type Item = T;
    type IntoIter = std::iter::Copied<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().copied()
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = CopiedProducer { base: left };
        let right = CopiedProducer { base: right };

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(CopiedFolder { base: folder }).base
    }
}
