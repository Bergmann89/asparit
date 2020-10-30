use super::{
    Consumer, IndexedConsumer, Producer, IndexedProducer, Reducer, ProducerCallback, IndexedProducerCallback,
};

pub trait Executor<D>
where D: Send,
{
    type Result: Send;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer,
        C: Consumer<P::Item, Result = D, Reducer = R>,
        R: Reducer<D>;

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer,
        C: IndexedConsumer<P::Item, Result = D, Reducer = R>,
        R: Reducer<D>;
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

impl<E, D, C, I, R> ProducerCallback<I> for ExecutorCallback<E, C>
where
    E: Executor<D>,
    D: Send,
    C: Consumer<I, Result = D, Reducer = R>,
    R: Reducer<D>,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = I>
    {
        self.executor.exec(producer, self.consumer)
    }
}

impl<E, D, C, I, R> IndexedProducerCallback<I> for ExecutorCallback<E, C>
where
    E: Executor<D>,
    D: Send,
    C: IndexedConsumer<I, Result = D, Reducer = R>,
    R: Reducer<D>,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = I>
    {
        self.executor.exec_indexed(producer, self.consumer)
    }
}
