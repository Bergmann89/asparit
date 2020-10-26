use crate::{
    core::{Collector, Executor},
    Consumer, Folder, ParallelIterator,
};

use super::noop::NoOpReducer;

pub struct ForEach<I, F> {
    iterator: I,
    operation: F,
}

impl<I, F> ForEach<I, F> {
    pub fn new(iterator: I, operation: F) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<I, F> Collector for ForEach<I, F>
where
    I: ParallelIterator,
    F: Fn(I::Item) + Sync + Send + Copy,
{
    type Iterator = I;
    type Consumer = ForEachConsumer<F>;

    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<Self::Iterator, Self::Consumer>,
    {
        let iterator = self.iterator;
        let operation = self.operation;

        let consumer = ForEachConsumer { operation };

        iterator.drive(executor, consumer)
    }
}

pub struct ForEachConsumer<F> {
    operation: F,
}

impl<F, T> Consumer<T> for ForEachConsumer<F>
where
    F: Fn(T) + Sync + Send + Copy,
{
    type Folder = ForEachConsumer<F>;
    type Reducer = NoOpReducer;
    type Result = ();

    fn split_off_left(&self) -> (Self, NoOpReducer) {
        (
            ForEachConsumer {
                operation: self.operation,
            },
            NoOpReducer,
        )
    }

    fn into_folder(self) -> Self {
        self
    }
}

impl<F, T> Folder<T> for ForEachConsumer<F>
where
    F: Fn(T) + Sync + Send + Copy,
{
    type Result = ();

    fn consume(self, item: T) -> Self {
        (self.operation)(item);

        self
    }

    fn consume_iter<I>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().for_each(self.operation);

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
        (0..10usize)
            .into_par_iter()
            .for_each(&|j| {
                println!("{}", j);
            })
            .exec();
    }
}
