use std::ops::Range;

use crate::{
    Consumer, Executor, Folder, IntoParallelIterator, ParallelIterator, Producer, ProducerCallback,
};

pub struct Iter {
    range: Range<usize>,
}

struct IterProducer {
    range: Range<usize>,
}

impl IntoParallelIterator for Range<usize> {
    type Iter = Iter;
    type Item = usize;

    fn into_par_iter(self) -> Self::Iter {
        Iter { range: self }
    }
}

impl ParallelIterator for Iter {
    type Item = usize;

    fn drive<E, C>(self, executor: E, consumer: C) -> E::Result
    where
        E: Executor<Self, C>,
        C: Consumer<Self::Item>,
    {
        executor.exec(self, consumer)
    }

    fn len_hint_opt(&self) -> Option<usize> {
        Some(self.range.end - self.range.start)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.callback(IterProducer { range: self.range })
    }
}

impl Producer for IterProducer {
    type Item = usize;

    fn split(mut self) -> (Self, Option<Self>) {
        let index = self.range.len() / 2;

        if index > 0 {
            let mid = self.range.start.wrapping_add(index);
            let right = mid..self.range.end;

            self.range.end = mid;

            (self, Some(IterProducer { range: right }))
        } else {
            (self, None)
        }
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        folder.consume_iter(self.range)
    }
}
