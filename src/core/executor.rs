use super::{Consumer, IndexedConsumer, IndexedParallelIterator, ParallelIterator};

pub trait Executor<I, C>
where
    I: ParallelIterator,
    C: Consumer<I::Item>,
{
    type Result;

    fn exec(self, iterator: I, consumer: C) -> Self::Result;
}

pub trait IndexedExecutor<I, C>
where
    I: IndexedParallelIterator,
    C: IndexedConsumer<I::Item>,
{
    type Result;

    fn exec_indexed(self, iterator: I, consumer: C) -> Self::Result;
}
