use std::cmp;
use std::mem::transmute;

use futures::{
    future::{BoxFuture, Future, FutureExt},
    join,
};
use tokio::task::spawn;

use crate::core::{
    Consumer, Executor, Folder, IndexedConsumer, IndexedProducer, Producer, Reducer,
};

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

impl<'a, D> Executor<'a, D> for Tokio
where
    D: Send,
{
    type Result = BoxFuture<'a, D>;

    fn exec<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: Producer + 'a,
        C: Consumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D> + Send,
    {
        let splits = producer.splits().unwrap_or(self.splits);
        let splitter = Splitter::new(splits);

        exec(splitter, producer, consumer)
    }

    fn exec_indexed<P, C, R>(self, producer: P, consumer: C) -> Self::Result
    where
        P: IndexedProducer + 'a,
        C: IndexedConsumer<P::Item, Result = D, Reducer = R> + 'a,
        R: Reducer<D> + Send,
    {
        let splits = producer.splits().unwrap_or(self.splits);
        let splitter = IndexedSplitter::new(
            splits,
            producer.len(),
            producer.min_len(),
            producer.max_len(),
        );

        exec_indexed(splitter, producer, consumer)
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
                    let ((left_consumer, reducer), right_consumer) =
                        (consumer.split_off_left(), consumer);

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
    C: IndexedConsumer<P::Item> + 'a,
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
        if self.splits > 0 {
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
