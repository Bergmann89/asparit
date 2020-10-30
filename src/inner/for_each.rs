use crate::{core::Driver, Consumer, Folder, ParallelIterator, Executor};

use super::noop::NoOpReducer;

pub struct ForEach<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> ForEach<X, O>
{
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<X, O> Driver<()> for ForEach<X, O>
where
    X: ParallelIterator,
    O: Fn(X::Item) + Clone + Send,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where E: Executor<()>
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

    #[test]
    fn test_for_each() {
        let x = (0..10usize)
            .into_par_iter()
            .map(Some)
            .for_each(|j| {
                println!("{:?}", j);
            })
            .exec();

        dbg!(x);
    }
}
