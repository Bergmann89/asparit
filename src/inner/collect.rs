use std::marker::PhantomData;

use crate::{core::{FromParallelIterator, Driver}, ParallelIterator, Executor};

pub struct Collect<X, T> {
    iterator: X,
    marker: PhantomData<T>,
}

impl<X, T> Collect<X, T> {
    pub fn new(iterator: X) -> Self {
        Self {
            iterator,
            marker: PhantomData,
        }
    }
}

impl<'a, X, T> Driver<'a, T> for Collect<X, T>
where
    X: ParallelIterator<'a>,
    T: FromParallelIterator<X::Item> + Send,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where E: Executor<'a, T>
    {
        T::from_par_iter(executor, self.iterator)
    }
}
