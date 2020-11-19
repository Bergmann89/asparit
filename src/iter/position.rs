use crate::{Driver, Executor, IndexedParallelIterator, WithIndexedProducer};

use super::find::{Find, FindMatch};

pub struct Position<X, O> {
    base: X,
    operation: O,
    find_match: FindMatch,
}

impl<X, O> Position<X, O> {
    pub fn new(base: X, operation: O, find_match: FindMatch) -> Self {
        Self {
            base,
            operation,
            find_match,
        }
    }
}

impl<'a, X, O, I> Driver<'a, Option<usize>, Option<(usize, bool)>> for Position<X, O>
where
    X: IndexedParallelIterator<'a, Item = I> + WithIndexedProducer<'a, Item = I>,
    O: Fn(I) -> bool + Clone + Send + 'a,
    I: Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, Option<usize>, Option<(usize, bool)>>,
    {
        let executor = executor.into_inner();
        let iterator = Find::new(
            self.base.map(self.operation).enumerate(),
            |(_, x): &(usize, bool)| -> bool { *x },
            self.find_match,
        );
        let inner = iterator.exec_with(executor);

        E::map(inner, |x| x.map(|(i, _)| i))
    }
}
