use crate::{core::Driver, Consumer, Executor, Folder, IndexedConsumer, ParallelIterator, Reducer};

pub struct Reduce<X, S, O> {
    iterator: X,
    identity: S,
    operation: O,
}

impl<X, S, O> Reduce<X, S, O> {
    pub fn new(iterator: X, identity: S, operation: O) -> Self {
        Self {
            iterator,
            identity,
            operation,
        }
    }
}

impl<'a, X, S, O> Driver<'a, X::Item> for Reduce<X, S, O>
where
    X: ParallelIterator<'a>,
    S: Fn() -> X::Item + Clone + Send + 'a,
    O: Fn(X::Item, X::Item) -> X::Item + Clone + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, X::Item>,
    {
        let iterator = self.iterator;
        let identity = self.identity;
        let operation = self.operation;

        let consumer = ReduceConsumer {
            identity,
            operation,
        };

        iterator.drive(executor, consumer)
    }
}

/* ReduceConsumer */

struct ReduceConsumer<S, O> {
    identity: S,
    operation: O,
}

impl<S, O> Clone for ReduceConsumer<S, O>
where
    S: Clone,
    O: Clone,
{
    fn clone(&self) -> Self {
        Self {
            identity: self.identity.clone(),
            operation: self.operation.clone(),
        }
    }
}

impl<S, O, T> Consumer<T> for ReduceConsumer<S, O>
where
    S: Fn() -> T + Clone + Send,
    O: Fn(T, T) -> T + Clone + Send,
    T: Send,
{
    type Folder = ReduceFolder<O, T>;
    type Reducer = Self;
    type Result = T;

    fn split_off_left(&self) -> (Self, Self::Reducer) {
        (self.clone(), self.clone())
    }

    fn into_folder(self) -> Self::Folder {
        ReduceFolder {
            operation: self.operation,
            item: (self.identity)(),
        }
    }
}

impl<S, O, T> IndexedConsumer<T> for ReduceConsumer<S, O>
where
    S: Fn() -> T + Clone + Send,
    O: Fn(T, T) -> T + Clone + Send,
    T: Send,
{
    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        (self.clone(), self.clone(), self)
    }
}

impl<S, O, T> Reducer<T> for ReduceConsumer<S, O>
where
    O: Fn(T, T) -> T,
    S: Fn() -> T,
    T: Send,
{
    fn reduce(self, left: T, right: T) -> T {
        (self.operation)(left, right)
    }
}

/* ReduceFolder */

struct ReduceFolder<O, T> {
    operation: O,
    item: T,
}

impl<O, T> Folder<T> for ReduceFolder<O, T>
where
    O: Fn(T, T) -> T + Clone,
{
    type Result = T;

    fn consume(mut self, item: T) -> Self {
        self.item = (self.operation)(self.item, item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        self.item = iter.into_iter().fold(self.item, self.operation.clone());

        self
    }

    fn complete(self) -> Self::Result {
        self.item
    }
}
