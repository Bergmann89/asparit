use crate::{Consumer, Folder, IndexedConsumer, Reducer};

pub struct NoOpConsumer;

impl<T> Consumer<T> for NoOpConsumer {
    type Folder = NoOpConsumer;
    type Reducer = NoOpReducer;
    type Result = ();

    fn split_off_left(&self) -> (Self, Self::Reducer) {
        (NoOpConsumer, NoOpReducer)
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

impl<T> IndexedConsumer<T> for NoOpConsumer {
    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        (NoOpConsumer, NoOpConsumer, NoOpReducer)
    }
}

pub struct NoOpReducer;

impl Reducer<()> for NoOpReducer {
    fn reduce(self, _left: (), _right: ()) {}
}
