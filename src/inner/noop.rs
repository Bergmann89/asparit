use crate::{Consumer, Folder, Reducer};

pub struct NoOpConsumer;

impl<T> Consumer<T> for NoOpConsumer {
    type Folder = NoOpConsumer;
    type Reducer = NoOpReducer;
    type Result = ();

    fn split(self) -> (Self, Self, Self::Reducer) {
        (NoOpConsumer, NoOpConsumer, NoOpReducer)
    }

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        (NoOpConsumer, NoOpConsumer, NoOpReducer)
    }

    fn into_folder(self) -> Self {
        self
    }

    fn is_full(&self) -> bool {
        false
    }
}

impl<T> Folder<T> for NoOpConsumer {
    type Result = ();

    fn consume(self, _item: T) -> Self {
        self
    }

    fn consume_iter<I>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().for_each(drop);
        self
    }

    fn complete(self) {}
}

pub struct NoOpReducer;

impl Reducer<()> for NoOpReducer {
    fn reduce(self, _left: (), _right: ()) {}
}
