use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, ParallelIterator, Producer, Reducer, Setup, WithIndexedProducer,
    WithSetup,
};

pub struct Rev<X> {
    base: X,
}

impl<X> Rev<X> {
    pub fn new(base: X) -> Self {
        Self { base }
    }
}

impl<'a, X, I> ParallelIterator<'a> for Rev<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    type Item = I;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, I> IndexedParallelIterator<'a> for Rev<X>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    I: Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_indexed_producer(ExecutorCallback::new(executor, consumer))
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

impl<'a, X> WithIndexedProducer<'a> for Rev<X>
where
    X: WithIndexedProducer<'a>,
{
    type Item = X::Item;

    fn with_indexed_producer<CB>(self, base: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_indexed_producer(RevCallback { base })
    }
}

/* RevCallback */

struct RevCallback<CB> {
    base: CB,
}

impl<'a, CB, I> IndexedProducerCallback<'a, I> for RevCallback<CB>
where
    CB: IndexedProducerCallback<'a, I>,
{
    type Output = CB::Output;

    fn callback<P>(self, base: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        self.base.callback(RevProducer { base })
    }
}

/* RevProducer */

struct RevProducer<P> {
    base: P,
}

impl<P> WithSetup for RevProducer<P>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<P> Producer for RevProducer<P>
where
    P: IndexedProducer,
{
    type Item = P::Item;
    type IntoIter = std::iter::Rev<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().rev()
    }

    fn split(self) -> (Self, Option<Self>) {
        let len = self.base.len();
        if len < 2 {
            return (self, None);
        }

        let (left, right) = self.split_at(len / 2);

        (left, Some(right))
    }
}

impl<P> IndexedProducer for RevProducer<P>
where
    P: IndexedProducer,
{
    type Item = P::Item;
    type IntoIter = std::iter::Rev<P::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        self.base.into_iter().rev()
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        (Self { base: right }, Self { base: left })
    }
}
