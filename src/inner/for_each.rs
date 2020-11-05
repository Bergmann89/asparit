use crate::{core::Driver, Consumer, Executor, Folder, ParallelIterator};

use super::noop::NoOpReducer;

pub struct ForEach<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> ForEach<X, O> {
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<'a, X, O> Driver<'a, ()> for ForEach<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(X::Item) + Clone + Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, ()>,
    {
        let iterator = self.iterator;
        let operation = self.operation;

        let consumer = ForEachConsumer { operation };

        iterator.drive(executor, consumer)
    }
}

pub struct ForEachConsumer<O> {
    operation: O,
}

impl<O, I> Consumer<I> for ForEachConsumer<O>
where
    O: Fn(I) + Clone + Send,
{
    type Folder = ForEachConsumer<O>;
    type Reducer = NoOpReducer;
    type Result = ();

    fn split_off_left(&self) -> (Self, NoOpReducer) {
        (
            ForEachConsumer {
                operation: self.operation.clone(),
            },
            NoOpReducer,
        )
    }

    fn into_folder(self) -> Self {
        self
    }
}

impl<O, I> Folder<I> for ForEachConsumer<O>
where
    O: Fn(I),
{
    type Result = ();

    fn consume(self, item: I) -> Self {
        (self.operation)(item);

        self
    }

    fn consume_iter<X>(self, iter: X) -> Self
    where
        X: IntoIterator<Item = I>,
    {
        iter.into_iter().for_each(&self.operation);

        self
    }

    fn complete(self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[tokio::test]
    async fn test_for_each() {
        use ::std::sync::atomic::{AtomicUsize, Ordering};
        use ::std::sync::Arc;

        let i = Arc::new(AtomicUsize::new(0));
        let j = Arc::new(AtomicUsize::new(0));

        let x = (0..10usize)
            .into_par_iter()
            .map_init(
                move || i.fetch_add(1, Ordering::Relaxed),
                |init, item| Some((*init, item)),
            )
            .for_each_init(
                move || j.fetch_add(1, Ordering::Relaxed),
                |init, item| {
                    println!("{:?} {:?}", init, item);
                },
            )
            .exec()
            .await;

        dbg!(x);
    }
}
