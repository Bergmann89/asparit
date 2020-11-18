use crate::{
    Consumer, Executor, Folder, ParallelIterator, Producer, ProducerCallback, Reducer, Setup,
    WithProducer, WithSetup,
};

/* Fold */

pub struct Fold<X, S, O> {
    base: X,
    init: S,
    operation: O,
}

impl<X, S, O> Fold<X, S, O> {
    pub fn new(base: X, init: S, operation: O) -> Self {
        Self {
            base,
            init,
            operation,
        }
    }
}

impl<'a, X, S, O, U> ParallelIterator<'a> for Fold<X, S, O>
where
    X: ParallelIterator<'a>,
    S: Fn() -> U + Clone + Send + 'a,
    O: Fn(U, X::Item) -> U + Clone + Send + 'a,
    U: Send + 'a,
{
    type Item = U;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            FoldConsumer {
                base: consumer,
                init: self.init,
                operation: self.operation,
            },
        )
    }
}

impl<'a, X, S, O, U> WithProducer<'a> for Fold<X, S, O>
where
    X: WithProducer<'a>,
    S: Fn() -> U + Clone + Send + 'a,
    O: Fn(U, X::Item) -> U + Clone + Send + 'a,
    U: Send + 'a,
{
    type Item = U;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(FoldCallback {
            base: callback,
            init: self.init,
            operation: self.operation,
        })
    }
}

/* FoldWith */

pub struct FoldWith<X, U, O> {
    base: X,
    init: U,
    operation: O,
}

impl<X, U, O> FoldWith<X, U, O> {
    pub fn new(base: X, init: U, operation: O) -> Self {
        Self {
            base,
            init,
            operation,
        }
    }
}

impl<'a, X, U, O> ParallelIterator<'a> for FoldWith<X, U, O>
where
    X: ParallelIterator<'a>,
    U: Clone + Send + 'a,
    O: Fn(U, X::Item) -> U + Clone + Send + 'a,
{
    type Item = U;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        let FoldWith {
            base,
            init,
            operation,
        } = self;

        base.drive(
            executor,
            FoldConsumer {
                base: consumer,
                init: move || init.clone(),
                operation,
            },
        )
    }
}

impl<'a, X, U, O> WithProducer<'a> for FoldWith<X, U, O>
where
    X: WithProducer<'a>,
    U: Clone + Send + 'a,
    O: Fn(U, X::Item) -> U + Clone + Send + 'a,
{
    type Item = U;

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        let FoldWith {
            base,
            init,
            operation,
        } = self;

        base.with_producer(FoldCallback {
            base: callback,
            init: move || init.clone(),
            operation,
        })
    }
}

/* FoldConsumer */

struct FoldConsumer<C, S, O> {
    base: C,
    init: S,
    operation: O,
}

impl<C, S, O> WithSetup for FoldConsumer<C, S, O>
where
    C: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, C, S, O, T, U> Consumer<T> for FoldConsumer<C, S, O>
where
    C: Consumer<U>,
    S: Fn() -> U + Clone + Send,
    O: Fn(U, T) -> U + Clone + Send,
{
    type Folder = FoldFolder<C::Folder, O, U>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = FoldConsumer {
            base: left,
            init: self.init.clone(),
            operation: self.operation.clone(),
        };
        let right = FoldConsumer {
            base: right,
            init: self.init,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = FoldConsumer {
            base: left,
            init: self.init.clone(),
            operation: self.operation.clone(),
        };
        let right = FoldConsumer {
            base: right,
            init: self.init,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        FoldFolder {
            base: self.base.into_folder(),
            item: (self.init)(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* FoldCallback */

struct FoldCallback<CB, S, O> {
    base: CB,
    init: S,
    operation: O,
}

impl<'a, CB, S, O, T, U> ProducerCallback<'a, T> for FoldCallback<CB, S, O>
where
    CB: ProducerCallback<'a, U>,
    S: Fn() -> U + Clone + Send + 'a,
    O: Fn(U, T) -> U + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(FoldProducer {
            base: producer,
            init: self.init,
            operation: self.operation,
        })
    }
}

/* FoldProducer */

struct FoldProducer<P, S, O> {
    base: P,
    init: S,
    operation: O,
}

impl<P, S, O> WithSetup for FoldProducer<P, S, O>
where
    P: WithSetup,
{
    fn setup(&self) -> Setup {
        self.base.setup()
    }
}

impl<'a, P, S, O, U> Producer for FoldProducer<P, S, O>
where
    P: Producer,
    S: Fn() -> U + Clone + Send,
    O: Fn(U, P::Item) -> U + Clone + Send,
{
    type Item = U;
    type IntoIter = std::iter::Once<U>;

    fn into_iter(self) -> Self::IntoIter {
        let item = (self.init)();
        let item = self.base.into_iter().fold(item, self.operation);

        std::iter::once(item)
    }

    fn split(self) -> (Self, Option<Self>) {
        let init = self.init;
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = FoldProducer {
            base: left,
            init: init.clone(),
            operation: operation.clone(),
        };
        let right = right.map(move |right| FoldProducer {
            base: right,
            init,
            operation,
        });

        (left, right)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base
            .fold_with(FoldFolder {
                base: folder,
                item: (self.init)(),
                operation: self.operation,
            })
            .base
    }
}

/* FoldFolder */

struct FoldFolder<F, O, U> {
    base: F,
    operation: O,
    item: U,
}

impl<F, O, U, T> Folder<T> for FoldFolder<F, O, U>
where
    F: Folder<U>,
    O: Fn(U, T) -> U + Clone,
{
    type Result = F::Result;

    fn consume(mut self, item: T) -> Self {
        self.item = (self.operation)(self.item, item);

        self
    }

    fn consume_iter<X>(self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        let FoldFolder {
            base,
            operation,
            item,
        } = self;
        let item = iter
            .into_iter()
            .take_while(|_| !base.is_full())
            .fold(item, operation.clone());

        FoldFolder {
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
