use crate::core::{Consumer, Executor, Folder, IndexedProducer, Producer, Reducer};

#[derive(Default)]
pub struct Sequential;

impl<'a, D> Executor<'a, D> for Sequential
where
    D: Send + 'a,
{
    type Result = D;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D>,
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
        C: Consumer<P::Item, Result = D, Reducer = R>,
        R: Reducer<D>,
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

    fn join<R>(left: D, right: D, reducer: R) -> Self::Result
    where
        R: Reducer<D> + Send,
    {
        reducer.reduce(left, right)
    }
}
