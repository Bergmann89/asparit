use crate::core::{
    Consumer, Executor, Folder, IndexedConsumer, IndexedExecutor, IndexedParallelIterator,
    IndexedProducer, IndexedProducerCallback, ParallelIterator, Producer, ProducerCallback,
};

#[derive(Default)]
pub struct Sequential;

struct Callback<C> {
    consumer: C,
}

struct IndexedCallback<C> {
    consumer: C,
}

impl<I, C> Executor<I, C> for Sequential
where
    I: ParallelIterator,
    C: Consumer<I::Item>,
{
    type Result = C::Result;

    fn exec(self, iterator: I, consumer: C) -> Self::Result {
        iterator.with_producer(Callback { consumer })
    }
}

impl<I, C> IndexedExecutor<I, C> for Sequential
where
    I: IndexedParallelIterator,
    C: IndexedConsumer<I::Item>,
{
    type Result = C::Result;

    fn exec_indexed(self, iterator: I, consumer: C) -> Self::Result {
        iterator.with_producer_indexed(IndexedCallback { consumer })
    }
}

impl<C, T> ProducerCallback<T> for Callback<C>
where
    C: Consumer<T>,
{
    type Output = C::Result;

    fn callback<P>(self, producer: P) -> C::Result
    where
        P: Producer<Item = T>,
    {
        if self.consumer.is_full() {
            self.consumer.into_folder().complete()
        } else {
            producer.fold_with(self.consumer.into_folder()).complete()
        }
    }
}

impl<C, T> IndexedProducerCallback<T> for IndexedCallback<C>
where
    C: IndexedConsumer<T>,
{
    type Output = C::Result;
    fn callback<P>(self, _producer: P) -> C::Result
    where
        P: IndexedProducer<Item = T>,
    {
        self.consumer.into_folder().complete()
    }
}
