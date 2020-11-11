use crate::{core::Driver, Consumer, Executor, Folder, ParallelExtend, ParallelIterator, Reducer};

/* Unzip */

pub struct Unzip<X> {
    iterator: X,
}

impl<X> Unzip<X> {
    pub fn new(iterator: X) -> Self {
        Self { iterator }
    }
}

impl<'a, X, P, T> Driver<'a, P, T> for Unzip<X>
where
    X: ParallelIterator<'a>,
    P: Default + Send + 'a,
    UnzipExtend<P>: ParallelExtend<'a, X::Item, T>,
    <<UnzipExtend<P> as ParallelExtend<'a, X::Item, T>>::Consumer as Consumer<X::Item>>::Reducer:
        Reducer<T> + Send,
    T: Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, P, T>,
    {
        let executor = executor.into_inner();
        let consumer = UnzipExtend(P::default()).into_consumer();

        let inner = self.iterator.drive(executor, consumer);

        E::map(inner, |inner| {
            let UnzipExtend(ret) = UnzipExtend::map_result(inner);

            ret
        })
    }
}

pub struct UnzipExtend<T>(T);
pub struct UnzipConsumer<T>(T);
pub struct UnzipFolder<T>(T);
pub struct UnzipReducer<T>(T);
pub struct UnzipResult<T>(T);

macro_rules! parallel_extend_tuple {
    (($($E:ident),*), ($($I:ident),*), ($($T:ident),*), ($($C:ident),*), ($($F:ident),*), ($($R:ident),*)) => {

        #[allow(non_snake_case)]
        impl<'a, $($E,)+ $($I,)+ $($T,)+> ParallelExtend<'a, ($($I,)+), UnzipResult<($($T,)+)>> for UnzipExtend<($($E,)+)>
        where
            $($E: ParallelExtend<'a, $I, $T> + Send,)+
            $(<$E::Consumer as Consumer<$I>>::Reducer: Reducer<$T>,)+
            $($I: Send + 'a,)+
            $($T: Send,)+
        {
            type Consumer = UnzipConsumer<($($E::Consumer,)+)>;

            fn into_consumer(self) -> Self::Consumer {
                let UnzipExtend(($($E,)+)) = self;

                UnzipConsumer(($($E.into_consumer(),)+))
            }

            fn map_result(inner: UnzipResult<($($T,)+)>) -> Self {
                let UnzipResult(($($T,)+)) = inner;

                UnzipExtend(($($E::map_result($T),)+))
            }
        }

        #[allow(non_snake_case)]
        impl<$($C,)+ $($I,)+> Consumer<($($I,)+)> for UnzipConsumer<($($C,)+)>
        where
            $($C: Consumer<$I>,)+
        {
            type Folder = UnzipFolder<($($C::Folder,)+)>;
            type Reducer = UnzipReducer<($($C::Reducer,)+)>;
            type Result = UnzipResult<($($C::Result,)+)>;

            fn split(self) -> (Self, Self, Self::Reducer) {
                let UnzipConsumer(($($C,)+)) = self;
                let ($($C,)+) = ($($C.split(),)+);

                (
                    UnzipConsumer(($($C.0,)+)), UnzipConsumer(($($C.1,)+)), UnzipReducer(($($C.2,)+))
                )
            }

            fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
                let UnzipConsumer(($($C,)+)) = self;
                let ($($C,)+) = ($($C.split_at(index),)+);

                (
                    UnzipConsumer(($($C.0,)+)), UnzipConsumer(($($C.1,)+)), UnzipReducer(($($C.2,)+))
                )
            }

            fn into_folder(self) -> Self::Folder {
                let UnzipConsumer(($($C,)+)) = self;

                UnzipFolder(($($C.into_folder(),)+))
            }

            fn is_full(&self) -> bool {
                let UnzipConsumer(($($C,)+)) = self;

                true $(&& $C.is_full())+
            }
        }

        #[allow(non_snake_case)]
        impl<$($F,)+ $($I,)+> Folder<($($I,)+)> for UnzipFolder<($($F,)+)>
        where
            $($F: Folder<$I>,)+
        {
            type Result = UnzipResult<($($F::Result,)+)>;

            fn consume(self, item: ($($I,)+)) -> Self {
                let UnzipFolder(($($F,)+)) = self;
                let ($($I,)+) = item;

                UnzipFolder(($($F.consume($I),)+))
            }

            fn complete(self) -> Self::Result {
                let UnzipFolder(($($F,)+)) = self;

                UnzipResult(($($F.complete(),)+))
            }

            fn is_full(&self) -> bool {
                let UnzipFolder(($($F,)+)) = self;

                $($F.is_full() &&)+ true
            }
        }

        #[allow(non_snake_case)]
        impl<$($R,)+ $($T,)+> Reducer<UnzipResult<($($T,)+)>> for UnzipReducer<($($R,)+)>
        where
            $($R: Reducer<$T>,)+
        {
            fn reduce(self, left: UnzipResult<($($T,)+)>, right: UnzipResult<($($T,)+)>) -> UnzipResult<($($T,)+)> {
                let UnzipReducer(($($R,)+)) = self;
                let UnzipResult(($($T,)+)) = left;
                let UnzipResult(($($E,)+)) = right;

                UnzipResult(($($R.reduce($T, $E),)+))
            }
        }
    };
}

parallel_extend_tuple!((E1, E2), (I1, I2), (T1, T2), (C1, C2), (F1, F2), (R1, R2));
parallel_extend_tuple!(
    (E1, E2, E3),
    (I1, I2, I3),
    (T1, T2, T3),
    (C1, C2, C3),
    (F1, F2, F3),
    (R1, R2, R3)
);
parallel_extend_tuple!(
    (E1, E2, E3, E4),
    (I1, I2, I3, I4),
    (T1, T2, T3, T4),
    (C1, C2, C3, C4),
    (F1, F2, F3, F4),
    (R1, R2, R3, R4)
);
parallel_extend_tuple!(
    (E1, E2, E3, E4, E5),
    (I1, I2, I3, I4, I5),
    (T1, T2, T3, T4, T5),
    (C1, C2, C3, C4, C5),
    (F1, F2, F3, F4, F5),
    (R1, R2, R3, R4, R5)
);
