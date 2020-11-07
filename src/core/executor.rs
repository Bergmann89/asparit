use super::{
    Consumer, IndexedProducer, IndexedProducerCallback, Producer, ProducerCallback, Reducer,
};

pub trait Executor<'a, D>: Sized
where
    D: Send + 'a,
{
    type Result: Send;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D> + Send + 'a;

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer + 'a,
        C: Consumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D> + Send + 'a;

    fn split(self) -> (Self, Self);

    fn join<R>(left: Self::Result, right: Self::Result, reducer: R) -> Self::Result
    where
        R: Reducer<D> + Send + 'a,
        D: 'a;
}

pub struct ExecutorCallback<E, C> {
    executor: E,
    consumer: C,
}

impl<E, C> ExecutorCallback<E, C> {
    pub fn new(executor: E, consumer: C) -> Self {
        Self { executor, consumer }
    }
}

impl<'a, E, D, C, I, R> ProducerCallback<'a, I> for ExecutorCallback<E, C>
where
    E: Executor<'a, D>,
    D: Send + 'a,
    C: Consumer<I, Result = D, Reducer = R> + 'a,
    R: Reducer<D> + Send + 'a,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a,
    {
        self.executor.exec(producer, self.consumer)
    }
}

impl<'a, E, D, C, I, R> IndexedProducerCallback<'a, I> for ExecutorCallback<E, C>
where
    E: Executor<'a, D>,
    D: Send + 'a,
    C: Consumer<I, Result = D, Reducer = R> + 'a,
    R: Reducer<D> + Send + 'a,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        self.executor.exec_indexed(producer, self.consumer)
    }
}
