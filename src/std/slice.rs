use crate::{
    Consumer, Executor, ExecutorCallback, IndexedParallelIterator, IndexedProducer,
    IndexedProducerCallback, IntoParallelIterator, ParallelIterator, Producer, ProducerCallback,
    Reducer, WithSetup,
};

impl<'a, T> IntoParallelIterator<'a> for &'a [T]
where
    T: Send + Sync,
{
    type Iter = Iter<'a, T>;
    type Item = &'a T;

    fn into_par_iter(self) -> Self::Iter {
        Iter { slice: self }
    }
}

impl<'a, T> IntoParallelIterator<'a> for &'a mut [T]
where
    T: Send + Sync,
{
    type Iter = IterMut<'a, T>;
    type Item = &'a mut T;

    fn into_par_iter(self) -> Self::Iter {
        IterMut { slice: self }
    }
}

impl<'a, T> IntoParallelIterator<'a> for &'a Vec<T>
where
    T: Send + Sync,
{
    type Iter = Iter<'a, T>;
    type Item = &'a T;

    fn into_par_iter(self) -> Self::Iter {
        Iter { slice: self }
    }
}

impl<'a, T> IntoParallelIterator<'a> for &'a mut Vec<T>
where
    T: Send + Sync,
{
    type Iter = IterMut<'a, T>;
    type Item = &'a mut T;

    fn into_par_iter(self) -> Self::Iter {
        IterMut { slice: self }
    }
}

/* Iter */

pub struct Iter<'a, T> {
    slice: &'a [T],
}

impl<'a, T> ParallelIterator<'a> for Iter<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(IterProducer { slice: self.slice })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        Some(self.slice.len())
    }
}

impl<'a, T> IndexedParallelIterator<'a> for Iter<'a, T>
where
    T: Send + Sync,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        callback.callback(IterProducer { slice: self.slice })
    }

    fn len_hint(&self) -> usize {
        self.slice.len()
    }
}

/* IterProducer */

struct IterProducer<'a, T> {
    slice: &'a [T],
}

impl<'a, T> WithSetup for IterProducer<'a, T> {}

impl<'a, T> Producer for IterProducer<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter()
    }

    fn split(self) -> (Self, Option<Self>) {
        if self.slice.len() < 2 {
            (self, None)
        } else {
            let mid = self.slice.len() / 2;
            let (left, right) = self.split_at(mid);

            (left, Some(right))
        }
    }
}

impl<'a, T> IndexedProducer for IterProducer<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter()
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.slice.split_at(index);

        let left = IterProducer { slice: left };
        let right = IterProducer { slice: right };

        (left, right)
    }
}

/* IterMut */

pub struct IterMut<'a, T> {
    slice: &'a mut [T],
}

impl<'a, T> ParallelIterator<'a> for IterMut<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a mut T;

    fn drive<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<'a, Self::Item>,
    {
        callback.callback(IterMutProducer { slice: self.slice })
    }

    fn len_hint_opt(&self) -> Option<usize> {
        Some(self.slice.len())
    }
}

impl<'a, T> IndexedParallelIterator<'a> for IterMut<'a, T>
where
    T: Send + Sync,
{
    fn drive_indexed<E, C, D, R>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<'a, D>,
        C: Consumer<Self::Item, Result = D, Reducer = R> + 'a,
        D: Send + 'a,
        R: Reducer<D> + Send + 'a,
    {
        self.with_producer_indexed(ExecutorCallback::new(executor, consumer))
    }

    fn with_producer_indexed<CB>(self, callback: CB) -> CB::Output
    where
        CB: IndexedProducerCallback<'a, Self::Item>,
    {
        callback.callback(IterMutProducer { slice: self.slice })
    }

    fn len_hint(&self) -> usize {
        self.slice.len()
    }
}

/* IterMutProducer */

struct IterMutProducer<'a, T> {
    slice: &'a mut [T],
}

impl<'a, T> WithSetup for IterMutProducer<'a, T> {}

impl<'a, T> Producer for IterMutProducer<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter_mut()
    }

    fn split(self) -> (Self, Option<Self>) {
        if self.slice.len() < 2 {
            (self, None)
        } else {
            let mid = self.slice.len() / 2;
            let (left, right) = self.split_at(mid);

            (left, Some(right))
        }
    }
}

impl<'a, T> IndexedProducer for IterMutProducer<'a, T>
where
    T: Send + Sync,
{
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter_mut()
    }

    fn len(&self) -> usize {
        self.slice.len()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.slice.split_at_mut(index);

        let left = IterMutProducer { slice: left };
        let right = IterMutProducer { slice: right };

        (left, right)
    }
}
