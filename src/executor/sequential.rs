use crate::core::{
    Consumer, Executor, Folder, IndexedConsumer,  IndexedProducer,
     Producer,  Reducer,
};

#[derive(Default)]
pub struct Sequential;

impl<'a, D> Executor<'a, D> for Sequential
where D: Send,
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
        C: IndexedConsumer<P::Item, Result = D, Reducer = R>,
        R: Reducer<D>,
    {
        if consumer.is_full() {
            consumer.into_folder().complete()
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }
}
