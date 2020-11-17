use std::marker::PhantomData;

use crate::{
    misc::Try, Consumer, Executor, Folder, ParallelIterator, Producer, ProducerCallback, Reducer,
    Setup, WithSetup,
};

/* TryFold */

pub struct TryFold<X, S, O, T> {
    base: X,
    init: S,
    operation: O,
    marker: PhantomData<T>,
}

impl<X, S, O, T> TryFold<X, S, O, T> {
    pub fn new(base: X, init: S, operation: O) -> Self {
        Self {
            base,
            init,
            operation,
            marker: PhantomData,
        }
    }
}

impl<'a, X, S, O, U, T> ParallelIterator<'a> for TryFold<X, S, O, T>
where
    X: ParallelIterator<'a>,
    S: Fn() -> U + Clone + Send + 'a,
    O: Fn(U, X::Item) -> T + Clone + Send + 'a,
    U: Send,
    T: Try<Ok = U> + Send + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            TryFoldConsumer {
                base: consumer,
                init: self.init,
                operation: self.operation,
                marker: PhantomData,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(TryFoldCallback {
            base: callback,
            init: self.init,
            operation: self.operation,
            marker: PhantomData,
        })
    }
}

/* TryFoldWith */

pub struct TryFoldWith<X, U, O, T> {
    base: X,
    init: U,
    operation: O,
    marker: PhantomData<T>,
}

impl<X, U, O, T> TryFoldWith<X, U, O, T> {
    pub fn new(base: X, init: U, operation: O) -> Self {
        Self {
            base,
            init,
            operation,
            marker: PhantomData,
        }
    }
}

impl<'a, X, U, O, T> ParallelIterator<'a> for TryFoldWith<X, U, O, T>
where
    X: ParallelIterator<'a>,
    U: Clone + Send + 'a,
    O: Fn(U, X::Item) -> T + Clone + Send + 'a,
    T: Try<Ok = U> + Send + 'a,
{
    type Item = T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let TryFoldWith {
            base,
            init,
            operation,
            ..
        } = self;

        base.drive(
            executor,
            TryFoldConsumer {
                base: consumer,
                init: move || init.clone(),
                operation,
                marker: PhantomData,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        let TryFoldWith {
            base,
            init,
            operation,
            ..
        } = self;

        base.with_producer(TryFoldCallback {
            base: callback,
            init: move || init.clone(),
            operation,
            marker: PhantomData,
        })
    }
}

/* TryFoldConsumer */

struct TryFoldConsumer<C, S, O, T> {
    base: C,
    init: S,
    operation: O,
    marker: PhantomData<T>,
}

impl<C, S, O, T> WithSetup for TryFoldConsumer<C, S, O, T>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, C, S, O, T, I> Consumer<I> for TryFoldConsumer<C, S, O, T>
where
    C: Consumer<T>,
    S: Fn() -> T::Ok + Clone + Send,
    O: Fn(T::Ok, I) -> T + Clone + Send,
    T: Try + Send,
{
    type Folder = TryFoldFolder<C::Folder, O, T>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = TryFoldConsumer {
            base: left,
            init: self.init.clone(),
            operation: self.operation.clone(),
            marker: PhantomData,
        };
        let right = TryFoldConsumer {
            base: right,
            init: self.init,
            operation: self.operation,
            marker: PhantomData,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = TryFoldConsumer {
            base: left,
            init: self.init.clone(),
            operation: self.operation.clone(),
            marker: PhantomData,
        };
        let right = TryFoldConsumer {
            base: right,
            init: self.init,
            operation: self.operation,
            marker: PhantomData,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        TryFoldFolder {
            base: self.base.into_folder(),
            item: T::from_ok((self.init)()),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* TryFoldCallback */

struct TryFoldCallback<CB, S, O, T> {
    base: CB,
    init: S,
    operation: O,
    marker: PhantomData<T>,
}

impl<'a, CB, S, O, T, I> ProducerCallback<'a, I> for TryFoldCallback<CB, S, O, T>
where
    CB: ProducerCallback<'a, T>,
    S: Fn() -> T::Ok + Clone + Send + 'a,
    O: Fn(T::Ok, I) -> T + Clone + Send + 'a,
    T: Try + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = I> + 'a,
    {
        self.base.callback(TryFoldProducer {
            base: producer,
            init: self.init,
            operation: self.operation,
            marker: PhantomData,
        })
    }
}

/* TryFoldProducer */

struct TryFoldProducer<P, S, O, T> {
    base: P,
    init: S,
    operation: O,
    marker: PhantomData<T>,
}

impl<P, S, O, T> WithSetup for TryFoldProducer<P, S, O, T>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, P, S, O, T> Producer for TryFoldProducer<P, S, O, T>
where
    P: Producer,
    S: Fn() -> T::Ok + Clone + Send,
    O: Fn(T::Ok, P::Item) -> T + Clone + Send,
    T: Try + Send,
{
    type Item = T;
    type IntoIter = std::iter::Once<T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut ret = T::from_ok((self.init)());

        for item in self.base.into_iter() {
            match ret.into_result() {
                Ok(value) => ret = (self.operation)(value, item),
                Err(err) => return std::iter::once(T::from_error(err)),
            }
        }

        std::iter::once(ret)
    }

    fn split(self) -> (Self, Option<Self>) {
        let init = self.init;
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = TryFoldProducer {
            base: left,
            init: init.clone(),
            operation: operation.clone(),
            marker: PhantomData,
        };
        let right = right.map(move |right| TryFoldProducer {
            base: right,
            init,
            operation,
            marker: PhantomData,
        });

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base
            .fold_with(TryFoldFolder {
                base: folder,
                item: T::from_ok((self.init)()),
                operation: self.operation,
            })
            .base
    }
}

/* TryFoldFolder */

struct TryFoldFolder<F, O, T> {
    base: F,
    operation: O,
    item: T,
}

impl<F, O, T, I> Folder<I> for TryFoldFolder<F, O, T>
where
    F: Folder<T>,
    O: Fn(T::Ok, I) -> T + Clone,
    T: Try + Send,
{
    type Result = F::Result;

    fn consume(mut self, item: I) -> Self {
        self.item = match self.item.into_result() {
            Ok(value) => (self.operation)(value, item),
            Err(err) => T::from_error(err),
        };

        self
    }

    fn consume_iter<X>(self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        let TryFoldFolder {
            base,
            operation,
            mut item,
        } = self;

        for next in iter.into_iter() {
            if base.is_full() {
                break;
            }

            match item.into_result() {
                Ok(value) => item = operation(value, next),
                Err(err) => {
                    item = T::from_error(err);

                    break;
                }
            }
        }

        TryFoldFolder {
            base,
            operation,
            item,
        }
    }

    fn complete(self) -> Self::Result {
        self.base.consume(self.item).complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}
