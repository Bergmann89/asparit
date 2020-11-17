use std::iter::{empty, once};
use std::marker::PhantomData;

use crate::{core::Driver, Consumer, Executor, Folder, ParallelIterator, Reducer, WithSetup};

/* Product */

pub struct Product<X, P> {
    iterator: X,
    marker: PhantomData<P>,
}

impl<X, P> Product<X, P> {
    pub fn new(iterator: X) -> Self {
        Self {
            iterator,
            marker: PhantomData,
        }
    }
}

impl<'a, X, P> Driver<'a, P> for Product<X, P>
where
    X: ParallelIterator<'a>,
    P: std::iter::Product<X::Item> + std::iter::Product + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, P>,
    {
        let iterator = self.iterator;
        let consumer = ProductConsumer(PhantomData);

        iterator.drive(executor, consumer)
    }
}

/* ProductConsumer */

pub struct ProductConsumer<P>(PhantomData<P>);

impl<P> WithSetup for ProductConsumer<P> {}

impl<P, I> Consumer<I> for ProductConsumer<P>
where
    P: std::iter::Product<I> + std::iter::Product + Send,
{
    type Folder = ProductFolder<P>;
    type Reducer = ProductReducer<P>;
    type Result = P;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let left = self;
        let right = ProductConsumer(PhantomData);

        (left, right, ProductReducer(PhantomData))
    }

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        let left = self;
        let right = ProductConsumer(PhantomData);

        (left, right, ProductReducer(PhantomData))
    }

    fn into_folder(self) -> Self::Folder {
        ProductFolder {
            product: empty::<I>().product(),
        }
    }
}

/* ProductFolder */

pub struct ProductFolder<P> {
    product: P,
}

impl<P, I> Folder<I> for ProductFolder<P>
where
    P: std::iter::Product<I> + std::iter::Product + Send,
{
    type Result = P;

    fn consume(mut self, item: I) -> Self {
        self.product = mul(self.product, once(item).product());

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        self.product = mul(self.product, iter.into_iter().product());

        self
    }

    fn complete(self) -> Self::Result {
        self.product
    }
}

/* ProductReducer */

pub struct ProductReducer<P>(PhantomData<P>);

impl<P> Reducer<P> for ProductReducer<P>
where
    P: std::iter::Product + Send,
{
    fn reduce(self, left: P, right: P) -> P {
        mul(left, right)
    }
}

fn mul<T>(left: T, right: T) -> T
where
    T: std::iter::Product,
{
    once(left).chain(once(right)).product()
}
