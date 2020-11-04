use crate::{Executor, DefaultExecutor};

pub trait Driver<'a, D>: Sized
where D: Send,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where E: Executor<'a, D>;

    fn exec(self) -> <DefaultExecutor as Executor<'a, D>>::Result
    { self.exec_with(DefaultExecutor::default()) }
}
