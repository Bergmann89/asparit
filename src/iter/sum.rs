use std::iter::{empty, once};
use std::marker::PhantomData;

use crate::{core::Driver, Consumer, Executor, Folder, ParallelIterator, Reducer, WithSetup};

/* Sum */

pub struct Sum<X, S> {
    iterator: X,
    marker: PhantomData<S>,
}

impl<X, S> Sum<X, S> {
    pub fn new(iterator: X) -> Self {
        Self {
            iterator,
            marker: PhantomData,
        }
    }
}

impl<'a, X, S> Driver<'a, S> for Sum<X, S>
where
    X: ParallelIterator<'a>,
    S: std::iter::Sum<X::Item> + std::iter::Sum + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, S>,
    {
        let iterator = self.iterator;
        let consumer = SumConsumer(PhantomData);

        iterator.drive(executor, consumer)
    }
}

/* SumConsumer */

pub struct SumConsumer<S>(PhantomData<S>);

impl<S> WithSetup for SumConsumer<S> {}

impl<S, I> Consumer<I> for SumConsumer<S>
where
    S: std::iter::Sum<I> + std::iter::Sum + Send,
{
    type Folder = SumFolder<S>;
    type Reducer = SumReducer<S>;
    type Result = S;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let left = self;
        let right = SumConsumer(PhantomData);

        (left, right, SumReducer(PhantomData))
    }

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        let left = self;
        let right = SumConsumer(PhantomData);

        (left, right, SumReducer(PhantomData))
    }

    fn into_folder(self) -> Self::Folder {
        SumFolder {
            sum: empty::<I>().sum(),
        }
    }
}

/* SumFolder */

pub struct SumFolder<S> {
    sum: S,
}

impl<S, I> Folder<I> for SumFolder<S>
where
    S: std::iter::Sum<I> + std::iter::Sum + Send,
{
    type Result = S;

    fn consume(mut self, item: I) -> Self {
        self.sum = add(self.sum, once(item).sum());

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        self.sum = add(self.sum, iter.into_iter().sum());

        self
    }

    fn complete(self) -> Self::Result {
        self.sum
    }
}

/* SumReducer */

pub struct SumReducer<S>(PhantomData<S>);

impl<S> Reducer<S> for SumReducer<S>
where
    S: std::iter::Sum + Send,
{
    fn reduce(self, left: S, right: S) -> S {
        add(left, right)
    }
}

fn add<T>(left: T, right: T) -> T
where
    T: std::iter::Sum,
{
    once(left).chain(once(right)).sum()
}
