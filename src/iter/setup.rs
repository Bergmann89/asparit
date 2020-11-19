use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer, Setup, WithIndexedProducer,
    WithProducer, WithSetup,
};

pub struct SetupIter<X> {
    base: X,
    setup: Setup,
}

impl<X> SetupIter<X> {
    pub fn new(base: X, setup: Setup) -> Self {
        Self { base, setup }
    }
}

impl<'a, X> ParallelIterator<'a> for SetupIter<X>
where
    X: ParallelIterator<'a>,
{
    type Item = X::Item;

    fn drive<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let setup = self.setup;
        let consumer = SetupIterConsumer { base, setup };

        self.base.drive(executor, consumer)
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X> IndexedParallelIterator<'a> for SetupIter<X>
where
    X: IndexedParallelIterator<'a>,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, base: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let setup = self.setup;
        let consumer = SetupIterConsumer { base, setup };

        self.base.drive_indexed(executor, consumer)
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

impl<'a, X> WithProducer<'a> for SetupIter<X>
where
    X: WithProducer<'a>,
{
    type Item = X::Item;

    fn with_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        let setup = self.setup;

        self.base.with_producer(SetupIterCallback { base, setup })
    }
}

impl<'a, X> WithIndexedProducer<'a> for SetupIter<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = X::Item;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        let setup = self.setup;

        self.base
            .with_indexed_producer(SetupIterCallback { base, setup })
    }
}

/* SetupIterCallback */

struct SetupIterCallback<CB> {
    base: CB,
    setup: Setup,
}

impl<'a, CB, I> ProducerCallback<'a, I> for SetupIterCallback<CB>
where
    CB: ProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a,
    {
        let setup = self.setup;

        self.base.callback(SetupIterProducer { base, setup })
    }
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for SetupIterCallback<CB>
where
    CB: IndexedProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        let setup = self.setup;

        self.base.callback(SetupIterProducer { base, setup })
    }
}

/* SetupIterProducer */

struct SetupIterProducer<P> {
    base: P,
    setup: Setup,
}

impl<P> WithSetup for SetupIterProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup().merge(self.setup.clone())
    }
}

impl<P> Producer for SetupIterProducer<P>
where
    P: Producer,
{
    type Item = P::Item;
    type IntoIter = P::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter()
    }

    fn split(self) -> (Self, Option<Self>) {
        let setup = self.setup;
        let (left, right) = self.base.split();

        let left = Self {
            base: left,
            setup: setup.clone(),
        };
        let right = right.map(|base| Self { base, setup });

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(folder)
    }
}

impl<P> IndexedProducer for SetupIterProducer<P>
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
        let setup = self.setup;
        let (left, right) = self.base.split_at(index);

        let left = Self {
            base: left,
            setup: setup.clone(),
        };
        let right = Self { base: right, setup };

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base.fold_with(folder)
    }
}

/* SetupIterConsumer */

struct SetupIterConsumer<C> {
    base: C,
    setup: Setup,
}

impl<C> WithSetup for SetupIterConsumer<C>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup().merge(self.setup.clone())
    }
}

impl<C, I> Consumer<I> for SetupIterConsumer<C>
where
    C: Consumer<I>,
{
    type Folder = C::Folder;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let setup = self.setup;
        let (left, right, reducer) = self.base.split();

        let left = Self {
            base: left,
            setup: setup.clone(),
        };
        let right = Self { base: right, setup };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let setup = self.setup;
        let (left, right, reducer) = self.base.split_at(index);

        let left = Self {
            base: left,
            setup: setup.clone(),
        };
        let right = Self { base: right, setup };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        self.base.into_folder()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
