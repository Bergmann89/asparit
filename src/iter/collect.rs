use std::marker::PhantomData;

use crate::{
    core::{Driver, FromParallelIterator},
    Executor, ParallelIterator,
};

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

impl<'a, X, T> Driver<'a, T, T::ExecutorItem2, T::ExecutorItem3> for Collect<X, T>
where
    X: ParallelIterator<'a>,
    T: FromParallelIterator<'a, X::Item> + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, T, T::ExecutorItem2, T::ExecutorItem3>,
    {
        T::from_par_iter(executor, self.iterator)
    }
}
