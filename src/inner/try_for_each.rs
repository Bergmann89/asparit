use crate::{misc::Try, Driver, Executor, ParallelIterator};

/* TryForEach */

pub struct TryForEach<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> TryForEach<X, O> {
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<'a, X, O, T> Driver<'a, T> for TryForEach<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(X::Item) -> T + Clone + Sync + Send + 'a,
    T: Try<Ok = ()> + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, T>,
    {
        let TryForEach {
            iterator,
            operation,
        } = self;

        fn ok<R: Try<Ok = ()>>(_: (), _: ()) -> R {
            R::from_ok(())
        }

        iterator
            .map(operation)
            .try_reduce(<()>::default, ok)
            .exec_with(executor)
    }
}

/* TryForEachWith */

pub struct TryForEachWith<X, S, O> {
    iterator: X,
    init: S,
    operation: O,
}

impl<X, S, O> TryForEachWith<X, S, O> {
    pub fn new(iterator: X, init: S, operation: O) -> Self {
        Self {
            iterator,
            init,
            operation,
        }
    }
}

impl<'a, X, S, O, T> Driver<'a, T> for TryForEachWith<X, S, O>
where
    X: ParallelIterator<'a>,
    S: Clone + Send + 'a,
    O: Fn(&mut S, X::Item) -> T + Clone + Sync + Send + 'a,
    T: Try<Ok = ()> + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, T>,
    {
        let TryForEachWith {
            iterator,
            init,
            operation,
        } = self;

        fn ok<R: Try<Ok = ()>>(_: (), _: ()) -> R {
            R::from_ok(())
        }

        iterator
            .map_with(init, operation)
            .try_reduce(<()>::default, ok)
            .exec_with(executor)
    }
}

/* TryForEachInit */

pub struct TryForEachInit<X, S, O> {
    iterator: X,
    init: S,
    operation: O,
}

impl<X, S, O> TryForEachInit<X, S, O> {
    pub fn new(iterator: X, init: S, operation: O) -> Self {
        Self {
            iterator,
            init,
            operation,
        }
    }
}

impl<'a, X, S, O, T, U> Driver<'a, T> for TryForEachInit<X, S, O>
where
    X: ParallelIterator<'a>,
    S: Fn() -> U + Clone + Send + Sync + 'a,
    O: Fn(&mut U, X::Item) -> T + Clone + Sync + Send + 'a,
    T: Try<Ok = ()> + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, T>,
    {
        let TryForEachInit {
            iterator,
            init,
            operation,
        } = self;

        fn ok<R: Try<Ok = ()>>(_: (), _: ()) -> R {
            R::from_ok(())
        }

        iterator
            .map_init(init, operation)
            .try_reduce(<()>::default, ok)
            .exec_with(executor)
    }
}
