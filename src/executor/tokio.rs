use std::cmp;
use std::mem::transmute;

use futures::{
    future::{BoxFuture, Future, FutureExt},
    join,
};
use tokio::task::spawn;

use crate::core::{Consumer, Executor, Folder, IndexedProducer, Producer, Reducer};

pub struct Tokio {
    splits: usize,
}

impl Tokio {
    pub fn new(splits: usize) -> Self {
        Self { splits }
    }
}

impl Default for Tokio {
    fn default() -> Self {
        Self {
            splits: 2 * num_cpus::get(),
        }
    }
}

impl<'a, T1, T2, T3> Executor<'a, T1, T2, T3> for Tokio
where
    T1: Send + 'a,
    T2: Send + 'a,
    T3: Send + 'a,
{
    type Result = BoxFuture<'a, T1>;
    type Inner = Tokio;

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
        async move {
            let left = left.await;
            let right = right.await;

            reducer.reduce(left, right)
        }
        .boxed()
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
        async move {
            let value = inner.await;

            operation(value)
        }
        .boxed()
    }
}

fn exec<'a, P, C>(mut splitter: Splitter, producer: P, consumer: C) -> BoxFuture<'a, C::Result>
where
    P: Producer + 'a,
    C: Consumer<P::Item> + 'a,
    C::Reducer: Send,
{
    async move {
        if consumer.is_full() {
            consumer.into_folder().complete()
        } else if splitter.try_split() {
            match producer.split() {
                (left_producer, Some(right_producer)) => {
                    let (left_consumer, right_consumer, reducer) = consumer.split();

                    let left = run_as_task(exec(splitter, left_producer, left_consumer));
                    let right = run_as_task(exec(splitter, right_producer, right_consumer));

                    let (left_result, right_result) = join!(left, right);

                    reducer.reduce(left_result, right_result)
                }
                (producer, None) => producer.fold_with(consumer.into_folder()).complete(),
            }
        } else {
            producer.fold_with(consumer.into_folder()).complete()
        }
    }
    .boxed()
}

fn exec_indexed<'a, P, C>(
    mut splitter: IndexedSplitter,
    producer: P,
    consumer: C,
) -> BoxFuture<'a, C::Result>
where
    P: IndexedProducer + 'a,
    C: Consumer<P::Item> + 'a,
    C::Reducer: Send,
{
    async move {
        if consumer.is_full() {
            consumer.into_folder().complete()
        } else {
            let len = producer.len();
            if splitter.try_split(len) {
                let mid = len / 2;

                let (left_producer, right_producer) = producer.split_at(mid);
                let (left_consumer, right_consumer, reducer) = consumer.split_at(mid);

                let left = run_as_task(exec_indexed(splitter, left_producer, left_consumer));
                let right = run_as_task(exec_indexed(splitter, right_producer, right_consumer));

                let (left_result, right_result) = join!(left, right);

                reducer.reduce(left_result, right_result)
            } else {
                producer.fold_with(consumer.into_folder()).complete()
            }
        }
    }
    .boxed()
}

async fn run_as_task<'a, T, F>(f: F) -> T
where
    T: Send + 'a,
    F: Future<Output = T> + Send + 'a,
{
    struct Pointer<T>(*mut T);

    unsafe impl<T> Send for Pointer<T> {}

    let mut result = None;
    let r = Pointer(&mut result as *mut _);

    let task: BoxFuture<'a, ()> = async move {
        unsafe {
            *r.0 = Some(f.await);
        }
    }
    .boxed();
    let task: BoxFuture<'static, ()> = unsafe { transmute(task) };

    spawn(task).await.expect("Error in tokio executor");

    result.unwrap()
}

#[derive(Clone, Copy)]
struct Splitter {
    splits: usize,
}

impl Splitter {
    #[inline]
    fn new(splits: usize) -> Self {
        Self { splits }
    }

    #[inline]
    fn try_split(&mut self) -> bool {
        if self.splits > 1 {
            self.splits /= 2;

            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy)]
struct IndexedSplitter {
    inner: Splitter,
    min: usize,
}

impl IndexedSplitter {
    #[inline]
    fn new(splits: usize, len: usize, min: Option<usize>, max: Option<usize>) -> Self {
        let min = min.unwrap_or_default();
        let mut ret = Self {
            inner: Splitter::new(splits),
            min: cmp::max(min, 1),
        };

        if let Some(max) = max {
            let min_splits = len / cmp::max(max, 1);
            if min_splits > ret.inner.splits {
                ret.inner.splits = min_splits;
            }
        }

        ret
    }

    #[inline]
    fn try_split(&mut self, len: usize) -> bool {
        len / 2 >= self.min && self.inner.try_split()
    }
}
