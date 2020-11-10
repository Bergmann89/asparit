use crate::{
    Consumer, Executor, Folder, IndexedParallelIterator, IndexedProducer, IndexedProducerCallback,
    ParallelIterator, Producer, ProducerCallback, Reducer,
};

/* Update */

pub struct Update<X, O> {
    base: X,
    operation: O,
}

impl<X, O> Update<X, O> {
    pub fn new(base: X, operation: O) -> Self {
        Self { base, operation }
    }
}

impl<'a, X, O> ParallelIterator<'a> for Update<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&mut X::Item) + Clone + Send + 'a,
{
    type Item = X::Item;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive(
            executor,
            UpdateConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer(UpdateCallback {
            base: callback,
            operation: self.operation,
        })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        self.base.len_hint_opt()
    }
}

impl<'a, X, O> IndexedParallelIterator<'a> for Update<X, O>
where
    X: IndexedParallelIterator<'a>,
    O: Fn(&mut X::Item) + Clone + Send + 'a,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.base.drive_indexed(
            executor,
            UpdateConsumer {
                base: consumer,
                operation: self.operation,
            },
        )
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        self.base.with_producer_indexed(UpdateCallback {
            base: callback,
            operation: self.operation,
        })
    }

    fn len_hint(&self) -> usize {
        self.base.len_hint()
    }
}

/* UpdateConsumer */

struct UpdateConsumer<C, O> {
    base: C,
    operation: O,
}

impl<'a, C, O, T> Consumer<T> for UpdateConsumer<C, O>
where
    C: Consumer<T>,
    O: Fn(&mut T) + Clone + Send,
{
    type Folder = UpdateFolder<C::Folder, O>;
    type Reducer = C::Reducer;
    type Result = C::Result;

    fn split(self) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split();

        let left = UpdateConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = UpdateConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
        let (left, right, reducer) = self.base.split_at(index);

        let left = UpdateConsumer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = UpdateConsumer {
            base: right,
            operation: self.operation,
        };

        (left, right, reducer)
    }

    fn into_folder(self) -> Self::Folder {
        UpdateFolder {
            base: self.base.into_folder(),
            operation: self.operation,
        }
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* UpdateFolder */

struct UpdateFolder<F, O> {
    base: F,
    operation: O,
}

impl<F, O, T> Folder<T> for UpdateFolder<F, O>
where
    F: Folder<T>,
    O: Fn(&mut T) + Clone,
{
    type Result = F::Result;

    fn consume(mut self, mut item: T) -> Self {
        (self.operation)(&mut item);

        self.base = self.base.consume(item);

        self
    }

    fn consume_iter<X>(mut self, iter: X) -> Self
    where
        X: IntoIterator<Item = T>,
    {
        self.base = self
            .base
            .consume_iter(iter.into_iter().map(apply(self.operation.clone())));

        self
    }

    fn complete(self) -> Self::Result {
        self.base.complete()
    }

    fn is_full(&self) -> bool {
        self.base.is_full()
    }
}

/* UpdateCallback */

struct UpdateCallback<CB, O> {
    base: CB,
    operation: O,
}

impl<'a, CB, O, T> ProducerCallback<'a, T> for UpdateCallback<CB, O>
where
    CB: ProducerCallback<'a, T>,
    O: Fn(&mut T) + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: Producer<Item = T> + 'a,
    {
        self.base.callback(UpdateProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

impl<'a, CB, O, T> IndexedProducerCallback<'a, T> for UpdateCallback<CB, O>
where
    CB: IndexedProducerCallback<'a, T>,
    O: Fn(&mut T) + Clone + Send + 'a,
{
    type Output = CB::Output;

    fn callback<P>(self, producer: P) -> Self::Output
    where
        P: IndexedProducer<Item = T> + 'a,
    {
        self.base.callback(UpdateProducer {
            base: producer,
            operation: self.operation,
        })
    }
}

/* UpdateProducer */

struct UpdateProducer<P, O> {
    base: P,
    operation: O,
}

impl<'a, P, O, T> Producer for UpdateProducer<P, O>
where
    P: Producer<Item = T>,
    O: Fn(&mut T) + Clone + Send,
{
    type Item = T;
    type IntoIter = UpdateIter<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        UpdateIter {
            base: self.base.into_iter(),
            operation: self.operation,
        }
    }

    fn split(self) -> (Self, Option<Self>) {
        let operation = self.operation;
        let (left, right) = self.base.split();

        let left = UpdateProducer {
            base: left,
            operation: operation.clone(),
        };
        let right = right.map(move |right| UpdateProducer {
            base: right,
            operation,
        });

        (left, right)
    }

    fn splits(&self) -> Option<usize> {
        self.base.splits()
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base
            .fold_with(UpdateFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}

impl<'a, P, O, T> IndexedProducer for UpdateProducer<P, O>
where
    P: IndexedProducer<Item = T>,
    O: Fn(&mut T) + Clone + Send,
{
    type Item = T;
    type IntoIter = UpdateIter<P::IntoIter, O>;

    fn into_iter(self) -> Self::IntoIter {
        UpdateIter {
            base: self.base.into_iter(),
            operation: self.operation,
        }
    }

    fn len(&self) -> usize {
        self.base.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.base.split_at(index);

        let left = UpdateProducer {
            base: left,
            operation: self.operation.clone(),
        };
        let right = UpdateProducer {
            base: right,
            operation: self.operation,
        };

        (left, right)
    }

    fn splits(&self) -> Option<usize> {
        self.base.splits()
    }

    fn min_len(&self) -> Option<usize> {
        self.base.min_len()
    }

    fn max_len(&self) -> Option<usize> {
        self.base.max_len()
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        self.base
            .fold_with(UpdateFolder {
                base: folder,
                operation: self.operation,
            })
            .base
    }
}

/* UpdateIter */

struct UpdateIter<X, O> {
    base: X,
    operation: O,
}

impl<X, O> Iterator for UpdateIter<X, O>
where
    X: Iterator,
    O: Fn(&mut X::Item),
{
    type Item = X::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let mut v = self.base.next()?;

        (self.operation)(&mut v);

        Some(v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.base.size_hint()
    }

    fn fold<S, G>(self, init: S, op: G) -> S
    where
        G: FnMut(S, Self::Item) -> S,
    {
        self.base.map(apply(self.operation)).fold(init, op)
    }

    fn collect<C>(self) -> C
    where
        C: std::iter::FromIterator<Self::Item>,
    {
        self.base.map(apply(self.operation)).collect()
    }
}

impl<X, O> ExactSizeIterator for UpdateIter<X, O>
where
    X: ExactSizeIterator,
    O: Fn(&mut X::Item),
{
}

impl<X, O> DoubleEndedIterator for UpdateIter<X, O>
where
    X: DoubleEndedIterator,
    O: Fn(&mut X::Item),
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let mut v = self.base.next_back()?;

        (self.operation)(&mut v);

        Some(v)
    }
}

fn apply<O, T>(operation: O) -> impl Fn(T) -> T
where
    O: Fn(&mut T),
{
    move |mut item| {
        operation(&mut item);

        item
    }
}
