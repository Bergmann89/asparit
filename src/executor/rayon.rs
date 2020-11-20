use rayon_core::{current_num_threads, join_context};

use crate::core::{Consumer, Executor, Folder, IndexedProducer, Producer, Reducer};

use super::misc::{IndexedSplitter, Splitter};

pub struct Rayon {
    splits: usize,
}

impl Rayon {
    pub fn new(splits: usize) -> Self {
        Self { splits }
    }
}

impl Default for Rayon {
    fn default() -> Self {
        Self {
            splits: current_num_threads(),
        }
    }
}

impl<'a, T1, T2, T3> Executor<'a, T1, T2, T3> for Rayon
where
    T1: Send + 'a,
    T2: Send + 'a,
    T3: Send + 'a,
{
    type Result = T1;
    type Inner = Rayon;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = T1, Reducer = R> + 'a,
        R: Reducer<T1> + Send + 'a,
    {
        let setup = producer.setup().merge(consumer.setup());
        let splits = setup.splits.unwrap_or(self.splits);
        let splitter = Splitter::new(splits);

        exec(splitter, producer, consumer)
    }

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer + 'a,
        C: Consumer<P::Item, Result = T1, Reducer = R> + 'a,
        R: Reducer<T1> + Send + 'a,
    {
        let setup = producer.setup().merge(consumer.setup());
        let splits = setup.splits.unwrap_or(self.splits);
        let splitter = IndexedSplitter::new(splits, producer.len(), setup.min_len, setup.max_len);

        exec_indexed(splitter, producer, consumer)
    }

    fn ready(self, value: T1) -> Self::Result {
        value
    }

    fn split(self) -> (Self, Self) {
        let mut left = self;
        let right = Self {
            splits: left.splits / 2,
        };

        left.splits -= right.splits;

        (left, right)
    }

    fn join<R>(left: Self::Result, right: Self::Result, reducer: R) -> Self::Result
    where
        R: Reducer<T1> + Send + 'a,
        T1: 'a,
    {
        reducer.reduce(left, right)
    }

    fn into_inner(self) -> Self::Inner {
        self
    }

    fn map<O>(
        inner: <Self::Inner as Executor<'a, T2, T3, ()>>::Result,
        mut operation: O,
    ) -> Self::Result
    where
        O: FnMut(T2) -> T1 + Send + 'a,
    {
        operation(inner)
    }
}

fn exec<'a, P, C>(mut splitter: Splitter, producer: P, consumer: C) -> C::Result
where
    P: Producer + 'a,
    C: Consumer<P::Item> + 'a,
    C::Reducer: Send,
{
    if consumer.is_full() {
        consumer.into_folder().complete()
    } else if splitter.try_split() {
        match producer.split() {
            (left_producer, Some(right_producer)) => {
                let (left_consumer, right_consumer, reducer) = consumer.split();

                let (left_result, right_result) = join_context(
                    |_| exec(splitter, left_producer, left_consumer),
                    |_| exec(splitter, right_producer, right_consumer),
                );

                reducer.reduce(left_result, right_result)
            }
            (producer, None) => producer.fold_with(consumer.into_folder()).complete(),
        }
    } else {
        producer.fold_with(consumer.into_folder()).complete()
    }
}

fn exec_indexed<'a, P, C>(mut splitter: IndexedSplitter, producer: P, consumer: C) -> C::Result
where
    P: IndexedProducer + 'a,
    C: Consumer<P::Item> + 'a,
    C::Reducer: Send,
{
    if consumer.is_full() {
        consumer.into_folder().complete()
    } else {
        let len = producer.len();
        if splitter.try_split(len) {
            let mid = len / 2;

            let (left_producer, right_producer) = producer.split_at(mid);
            let (left_consumer, right_consumer, reducer) = consumer.split_at(mid);

            let (left_result, right_result) = join_context(
                |_| exec_indexed(splitter, left_producer, left_consumer),
                |_| exec_indexed(splitter, right_producer, right_consumer),
            );

            reducer.reduce(left_result, right_result)
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }
}
