use crate::{Driver, Executor, ParallelIterator};

pub struct Count<X> {
    iterator: X,
}

impl<X> Count<X> {
    pub fn new(iterator: X) -> Self {
        Self { iterator }
    }
}

impl<'a, X> Driver<'a, usize> for Count<X>
where
    X: ParallelIterator<'a>,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, usize>,
    {
        self.iterator.map(|_| 1).sum().exec_with(executor)
    }
}
