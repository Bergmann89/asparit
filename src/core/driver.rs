use crate::{Executor, DefaultExecutor};

pub trait Driver<D>: Sized
where D: Send,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where E: Executor<D>;

    fn exec(self) -> <DefaultExecutor as Executor<D>>::Result
    { self.exec_with(DefaultExecutor::default()) }
}
