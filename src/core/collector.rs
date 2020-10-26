use crate::{
    Consumer, DefaultExecutor, Executor, IndexedConsumer, IndexedExecutor, IndexedParallelIterator,
    ParallelIterator,
};

pub trait Collector: Sized {
    type Iterator: ParallelIterator;
    type Consumer: Consumer<<Self::Iterator as ParallelIterator>::Item>;

    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<Self::Iterator, Self::Consumer>;

    fn exec(self) -> <DefaultExecutor as Executor<Self::Iterator, Self::Consumer>>::Result {
        self.exec_with(DefaultExecutor::default())
    }
}

pub trait IndexedCollector: Sized {
    type Iterator: IndexedParallelIterator;
    type Consumer: IndexedConsumer<<Self::Iterator as ParallelIterator>::Item>;

    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: IndexedExecutor<Self::Iterator, Self::Consumer>;

    fn exec(self) -> <DefaultExecutor as Executor<Self::Iterator, Self::Consumer>>::Result {
        self.exec_with(DefaultExecutor::default())
    }
}
