use crate::core::{Consumer, Executor, Folder, IndexedProducer, Producer, Reducer};

#[derive(Default)]
pub struct Sequential;

impl<'a, T1, T2, T3> Executor<'a, T1, T2, T3> for Sequential
where
    T1: Send + 'a,
    T2: Send + 'a,
    T3: Send + 'a,
{
    type Result = T1;
    type Inner = Sequential;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = T1, Reducer = R> + 'a,
        R: Reducer<T1>,
    {
        if consumer.is_full() {
            consumer.into_folder().complete()
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer,
        C: Consumer<P::Item, Result = T1, Reducer = R>,
        R: Reducer<T1>,
    {
        if consumer.is_full() {
            consumer.into_folder().complete()
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }

    fn split(self) -> (Self, Self) {
        (Self, Self)
    }

    fn join<R>(left: T1, right: T1, reducer: R) -> Self::Result
    where
        R: Reducer<T1> + Send,
    {
        reducer.reduce(left, right)
    }

    fn into_inner(self) -> Self::Inner {
        self
    }

    fn map<O>(
        inner: <Self::Inner as Executor<'a, T2, T3, ()>>::Result,
        operation: O,
    ) -> Self::Result
    where
        O: Fn(T2) -> T1,
    {
        operation(inner)
    }
}
