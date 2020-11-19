use std::cmp::{min, Ord, Ordering};

use crate::{Driver, Executor, ParallelIterator};

/* Min */

pub struct Min<X> {
    iterator: X,
}

impl<X> Min<X> {
    pub fn new(iterator: X) -> Self {
        Self { iterator }
    }
}

impl<'a, X, T> Driver<'a, Option<T>> for Min<X>
where
    X: ParallelIterator<'a, Item = T>,
    T: Send + Ord + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Option<T>>,
    {
        self.iterator.reduce_with(min).exec_with(executor)
    }
}

/* MinBy */

pub struct MinBy<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> MinBy<X, O> {
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<'a, X, O, T> Driver<'a, Option<T>> for MinBy<X, O>
where
    X: ParallelIterator<'a, Item = T>,
    O: Fn(&T, &T) -> Ordering + Clone + Send + Sync + 'a,
    T: Send + Ord + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Option<T>>,
    {
        let operation = self.operation;

        self.iterator
            .reduce_with(move |a, b| match operation(&a, &b) {
                Ordering::Less => a,
                _ => b,
            })
            .exec_with(executor)
    }
}

/* MinByKey */

pub struct MinByKey<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> MinByKey<X, O> {
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<'a, X, O, K> Driver<'a, Option<X::Item>, Option<(K, X::Item)>> for MinByKey<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&X::Item) -> K + Clone + Send + Sync + 'a,
    K: Send + Ord + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Option<X::Item>, Option<(K, X::Item)>>,
    {
        let operation = self.operation;
        let executor = executor.into_inner();

        let ret = self
            .iterator
            .map(move |x| (operation(&x), x))
            .reduce_with(|a, b| match (a.0).cmp(&b.0) {
                Ordering::Less => a,
                _ => b,
            })
            .exec_with(executor);

        E::map(ret, |x| x.map(|x| x.1))
    }
}
