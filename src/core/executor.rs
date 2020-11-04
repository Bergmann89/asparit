use super::{
    Consumer, IndexedConsumer, Producer, IndexedProducer, Reducer, ProducerCallback, IndexedProducerCallback,
};

pub trait Executor<'a, D>
where D: Send,
{
    type Result: Send;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D> + Send;

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer + 'a,
        C: IndexedConsumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D> + Send;
}

pub struct ExecutorCallback<E, C> {
    executor: E,
    consumer: C,
}

impl<E, C> ExecutorCallback<E, C> {
    pub fn new(executor: E, consumer: C) -> Self {
        Self {
            executor,
            consumer,
        }
    }
}

impl<'a, E, D, C, I, R> ProducerCallback<'a, I> for ExecutorCallback<E, C>
where
    E: Executor<'a, D>,
    D: Send,
    C: Consumer<I, Result = D, Reducer = R> + 'a,
    R: Reducer<D> + Send,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a
    {
        self.executor.exec(producer, self.consumer)
    }
}

impl<'a, E, D, C, I, R> IndexedProducerCallback<'a, I> for ExecutorCallback<E, C>
where
    E: Executor<'a, D>,
    D: Send,
    C: IndexedConsumer<I, Result = D, Reducer = R> + 'a,
    R: Reducer<D> + Send,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a
    {
        self.executor.exec_indexed(producer, self.consumer)
    }
}
