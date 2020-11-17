use crate::{
    core::Driver, Consumer, Executor, Folder, ParallelExtend, ParallelIterator, Reducer, Setup,
    WithSetup,
};

/* Partition */

pub struct Partition<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> Partition<X, O> {
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<'a, X, O, P, T> Driver<'a, P, T> for Partition<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(&X::Item) -> bool + Clone,
    P: Default + Send + 'a,
    PartitionExtend<P, MapTwoFn<O>>: ParallelExtend<'a, X::Item, T>,
    <<PartitionExtend<P, MapTwoFn<O>> as ParallelExtend<'a, X::Item, T>>::Consumer as Consumer<
        X::Item,
    >>::Reducer: Reducer<T> + Send,
    T: Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, P, T>,
    {
        let operation = self.operation;
        let executor = executor.into_inner();
        let consumer = PartitionExtend {
            base: P::default(),
            operation: Some(MapTwoFn(operation)),
        }
        .into_consumer();

        let inner = self.iterator.drive(executor, consumer);

        E::map(inner, |inner| {
            let ret = PartitionExtend::map_result(inner);

            ret.base
        })
    }
}

/* PartitionMap */

pub struct PartitionMap<X, O> {
    iterator: X,
    operation: O,
}

impl<X, O> PartitionMap<X, O> {
    pub fn new(iterator: X, operation: O) -> Self {
        Self {
            iterator,
            operation,
        }
    }
}

impl<'a, X, O, P, T, R> Driver<'a, P, T> for PartitionMap<X, O>
where
    X: ParallelIterator<'a>,
    O: Fn(X::Item) -> R,
    P: Default + Send + 'a,
    PartitionExtend<P, MapAnyFn<O>>: ParallelExtend<'a, X::Item, T>,
    <<PartitionExtend<P, MapAnyFn<O>> as ParallelExtend<'a, X::Item, T>>::Consumer as Consumer<
        X::Item,
    >>::Reducer: Reducer<T> + Send,
    T: Send + 'a,
{
    fn exec_with<E>(self, executor: E) -> E::Result
    where
        E: Executor<'a, P, T>,
    {
        let operation = self.operation;
        let executor = executor.into_inner();
        let consumer = PartitionExtend {
            base: P::default(),
            operation: Some(MapAnyFn(operation)),
        }
        .into_consumer();

        let inner = self.iterator.drive(executor, consumer);

        E::map(inner, |inner| {
            let ret = PartitionExtend::map_result(inner);

            ret.base
        })
    }
}

/* Misc */

pub struct PartitionExtend<E, O> {
    base: E,
    operation: Option<O>,
}

pub struct PartitionConsumer<C, O> {
    base: C,
    operation: O,
}

pub struct PartitionFolder<F, O> {
    base: F,
    operation: O,
}

pub struct PartitionReducer<R> {
    base: R,
}

pub struct PartitionResult<T> {
    base: T,
}

pub trait PartitionFn<I> {
    type Output;

    fn call(&self, item: I) -> Self::Output;
}

macro_rules! parallel_extend_tuple {
    (($($E:ident),*), ($($I:ident),*), ($($T:ident),*), ($($C:ident),*), ($($F:ident),*), ($($R:ident),*)) => {

        #[allow(non_snake_case)]
        impl<'a, O, I, $($E,)+ $($I,)+ $($T,)+> ParallelExtend<'a, I, PartitionResult<($($T,)+)>> for PartitionExtend<($($E,)+), O>
        where
            O: PartitionFn<I, Output = ($(Option<$I>,)+)> + Clone + Send + 'a,
            I: Send + 'a,
            $($E: ParallelExtend<'a, $I, $T> + Send,)+
            $(<$E::Consumer as Consumer<$I>>::Reducer: Reducer<$T>,)+
            $($I: Send + 'a,)+
            $($T: Send,)+
        {
            type Consumer = PartitionConsumer<($($E::Consumer,)+), O>;

            fn into_consumer(self) -> Self::Consumer {
                let ($($E,)+) = self.base;
                let operation = self.operation;

                PartitionConsumer {
                    base: ($($E.into_consumer(),)+),
                    operation: operation.unwrap(),
                }
            }

            fn map_result(inner: PartitionResult<($($T,)+)>) -> Self {
                let ($($T,)+) = inner.base;

                PartitionExtend {
                    base: ($($E::map_result($T),)+),
                    operation: None,
                }
            }
        }

        #[allow(non_snake_case)]
        impl<O, $($C,)+> WithSetup for PartitionConsumer<($($C,)+), O>
        where
            $($C: WithSetup,)+
        {
            fn setup(&self) -> Setup {
                let ($($C,)+) = &self.base;

                let mut ret = Setup::default();
                $(ret = ret.merge($C.setup());)+

                ret
            }
        }

        #[allow(non_snake_case)]
        impl<O, I, $($I,)+ $($C,)+> Consumer<I> for PartitionConsumer<($($C,)+), O>
        where
            O: PartitionFn<I, Output = ($(Option<$I>,)+)> + Clone + Send,
            I: Send,
            $($I: Send,)+
            $($C: Consumer<$I>,)+
        {
            type Folder = PartitionFolder<($($C::Folder,)+), O>;
            type Reducer = PartitionReducer<($($C::Reducer,)+)>;
            type Result = PartitionResult<($($C::Result,)+)>;

            fn split(self) -> (Self, Self, Self::Reducer) {
                let operation = self.operation;
                let ($($C,)+) = self.base;
                let ($($C,)+) = ($($C.split(),)+);

                let left = PartitionConsumer {
                    base: ($($C.0,)+),
                    operation: operation.clone(),
                };
                let right = PartitionConsumer {
                    base: ($($C.1,)+),
                    operation,
                };
                let reducer = PartitionReducer {
                    base: ($($C.2,)+),
                };

                (left, right, reducer)
            }

            fn split_at(self, index: usize) -> (Self, Self, Self::Reducer) {
                let operation = self.operation;
                let ($($C,)+) = self.base;
                let ($($C,)+) = ($($C.split_at(index),)+);

                let left = PartitionConsumer {
                    base: ($($C.0,)+),
                    operation: operation.clone(),
                };
                let right = PartitionConsumer {
                    base: ($($C.1,)+),
                    operation,
                };
                let reducer = PartitionReducer {
                    base: ($($C.2,)+),
                };

                (left, right, reducer)
            }

            fn into_folder(self) -> Self::Folder {
                let ($($C,)+) = self.base;

                PartitionFolder {
                    base: ($($C.into_folder(),)+),
                    operation: self.operation,
                }
            }

            fn is_full(&self) -> bool {
                let ($($C,)+) = &self.base;

                true $(&& $C.is_full())+
            }
        }

        #[allow(non_snake_case)]
        impl<O, I, $($I,)+ $($F,)+> Folder<I> for PartitionFolder<($($F,)+), O>
        where
            O: PartitionFn<I, Output = ($(Option<$I>,)+)>,
            $($F: Folder<$I>,)+
        {
            type Result = PartitionResult<($($F::Result,)+)>;

            fn consume(self, item: I) -> Self {
                let operation = self.operation;
                let ($($I,)+) = operation.call(item);
                let ($(mut $F,)+) = self.base;

                $(
                    if let Some(item) = $I {
                        $F = $F.consume(item);
                    }
                )+

                PartitionFolder {
                    base: ($($F,)+),
                    operation,
                }
            }

            fn complete(self) -> Self::Result {
                let ($($F,)+) = self.base;

                PartitionResult {
                    base: ($($F.complete(),)+),
                }
            }

            fn is_full(&self) -> bool {
                let ($($F,)+) = &self.base;

                $($F.is_full() &&)+ true
            }
        }

        #[allow(non_snake_case)]
        impl<$($R,)+ $($T,)+> Reducer<PartitionResult<($($T,)+)>> for PartitionReducer<($($R,)+)>
        where
            $($R: Reducer<$T>,)+
        {
            fn reduce(self, left: PartitionResult<($($T,)+)>, right: PartitionResult<($($T,)+)>) -> PartitionResult<($($T,)+)> {
                let ($($R,)+) = self.base;
                let ($($T,)+) = left.base;
                let ($($E,)+) = right.base;

                PartitionResult {
                    base: ($($R.reduce($T, $E),)+),
                }
            }
        }
    };
}

/* MapTwoFn */

pub struct MapTwoFn<O>(O);

impl<O> Clone for MapTwoFn<O>
where
    O: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<O, I> PartitionFn<I> for MapTwoFn<O>
where
    O: Fn(&I) -> bool,
{
    type Output = (Option<I>, Option<I>);

    fn call(&self, item: I) -> Self::Output {
        if (self.0)(&item) {
            (Some(item), None)
        } else {
            (None, Some(item))
        }
    }
}

/* MapAnyFn */

pub struct MapAnyFn<O>(O);

impl<O> Clone for MapAnyFn<O>
where
    O: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<O, I, T> PartitionFn<I> for MapAnyFn<O>
where
    O: Fn(I) -> T,
{
    type Output = T;

    fn call(&self, item: I) -> Self::Output {
        (self.0)(item)
    }
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
