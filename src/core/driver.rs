#[cfg(feature = "default-executor")]
use crate::DefaultExecutor;
use crate::Executor;

pub trait Driver<'a, T1, T2 = (), T3 = ()>: Sized
where
    T1: Send + 'a,
    T2: Send + 'a,
    T3: Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, T1, T2, T3>;

    #[cfg(feature = "default-executor")]
    fn exec(self) -> <DefaultExecutor as Executor<'a, T1, T2, T3>>::Result {
        self.exec_with(DefaultExecutor::default())
    }
}
