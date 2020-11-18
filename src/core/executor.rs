use super::{
    Consumer, IndexedProducer, IndexedProducerCallback, Producer, ProducerCallback, Reducer,
};

pub trait Executor<'a, T1, T2 = (), T3 = ()>: Sized
where
    T1: Send + 'a,
    T2: Send + 'a,
    T3: Send + 'a,
{
    type Result: Send;
    type Inner: Executor<'a, T2, T3, ()>;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = T1, Reducer = R> + 'a,
        R: Reducer<T1> + Send + 'a;

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer + 'a,
        C: Consumer<P::Item, Result = T1, Reducer = R> + 'a,
        R: Reducer<T1> + Send + 'a;

    fn ready(self, value: T1) -> Self::Result;

    fn split(self) -> (Self, Self);

    fn join<R>(left: Self::Result, right: Self::Result, reducer: R) -> Self::Result
    where
        R: Reducer<T1> + Send + 'a,
        T1: 'a;

    fn into_inner(self) -> Self::Inner;

    fn map<O>(
        inner: <Self::Inner as Executor<'a, T2, T3, ()>>::Result,
        operation: O,
    ) -> Self::Result
    where
        O: FnMut(T2) -> T1 + Send + 'a;
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

impl<'a, E, T1, C, I, R> ProducerCallback<'a, I> for ExecutorCallback<E, C>
where
    E: Executor<'a, T1>,
    T1: Send + 'a,
    C: Consumer<I, Result = T1, Reducer = R> + 'a,
    R: Reducer<T1> + Send + 'a,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a,
    {
        self.executor.exec(producer, self.consumer)
    }
}

impl<'a, E, T1, C, I, R> IndexedProducerCallback<'a, I> for ExecutorCallback<E, C>
where
    E: Executor<'a, T1>,
    T1: Send + 'a,
    C: Consumer<I, Result = T1, Reducer = R> + 'a,
    R: Reducer<T1> + Send + 'a,
{
    type Output = E::Result;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = I> + 'a,
    {
        self.executor.exec_indexed(producer, self.consumer)
    }
}
