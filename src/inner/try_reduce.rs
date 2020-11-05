use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::{core::Driver, misc::Try, Consumer, Executor, Folder, ParallelIterator, Reducer};

pub struct TryReduce<X, S, O> {
    iterator: X,
    identity: S,
    operation: O,
}

impl<X, S, O> TryReduce<X, S, O> {
    pub fn new(iterator: X, identity: S, operation: O) -> Self {
        Self {
            iterator,
            identity,
            operation,
        }
    }
}

impl<'a, X, S, O, T> Driver<'a, T> for TryReduce<X, S, O>
where
    X: ParallelIterator<'a, Item = T>,
    S: Fn() -> T::Ok + Clone + Send + 'a,
    O: Fn(T::Ok, T::Ok) -> T + Clone + Send + 'a,
    T: Try + Send,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, X::Item>,
    {
        let iterator = self.iterator;
        let identity = self.identity;
        let operation = self.operation;

        let consumer = TryReduceConsumer {
            identity,
            operation,
            is_full: Arc::new(AtomicBool::new(false)),
        };

        iterator.drive(executor, consumer)
    }
}

/* TryReduceConsumer */

struct TryReduceConsumer<S, O> {
    identity: S,
    operation: O,
    is_full: Arc<AtomicBool>,
}

impl<S, O> Clone for TryReduceConsumer<S, O>
where
    S: Clone,
    O: Clone,
{
    fn clone(&self) -> Self {
        Self {
            identity: self.identity.clone(),
            operation: self.operation.clone(),
            is_full: self.is_full.clone(),
        }
    }
}

impl<S, O, T> Consumer<T> for TryReduceConsumer<S, O>
where
    S: Fn() -> T::Ok + Clone + Send,
    O: Fn(T::Ok, T::Ok) -> T + Clone + Send,
    T: Try + Send,
{
    type Folder = TryReduceFolder<O, T>;
    type Reducer = Self;
    type Result = T;

    fn split(self) -> (Self, Self, Self::Reducer) {
        (self.clone(), self.clone(), self)
    }

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        (self.clone(), self.clone(), self)
    }

    fn into_folder(self) -> Self::Folder {
        TryReduceFolder {
            operation: self.operation,
            item: Ok((self.identity)()),
            is_full: self.is_full,
        }
    }
}

impl<S, O, T> Reducer<T> for TryReduceConsumer<S, O>
where
    O: Fn(T::Ok, T::Ok) -> T,
    S: Fn() -> T::Ok,
    T: Try + Send,
{
    fn reduce(self, left: T, right: T) -> T {
        match (left.into_result(), right.into_result()) {
            (Ok(left), Ok(right)) => (self.operation)(left, right),
            (Err(e), _) | (_, Err(e)) => T::from_error(e),
        }
    }
}

/* TryReduceFolder */

struct TryReduceFolder<O, T>
where
    T: Try,
{
    operation: O,
    item: Result<T::Ok, T::Error>,
    is_full: Arc<AtomicBool>,
}

impl<O, T> Folder<T> for TryReduceFolder<O, T>
where
    O: Fn(T::Ok, T::Ok) -> T + Clone,
    T: Try,
{
    type Result = T;

    fn consume(mut self, item: T) -> Self {
        if let Ok(left) = self.item {
            self.item = match item.into_result() {
                Ok(right) => (self.operation)(left, right).into_result(),
                Err(error) => Err(error),
            };
        }

        if self.item.is_err() {
            self.is_full.store(true, Ordering::Relaxed)
        }

        self
    }

    fn complete(self) -> Self::Result {
        match self.item {
            Ok(v) => T::from_ok(v),
            Err(v) => T::from_error(v),
        }
    }

    fn is_full(&self) -> bool {
        self.is_full.load(Ordering::Relaxed)
    }
}
