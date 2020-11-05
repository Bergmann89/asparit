pub trait Try {
    type Ok;
    type Error;

    fn into_result(self) -> Result<Self::Ok, Self::Error>;
    fn from_ok(v: Self::Ok) -> Self;
    fn from_error(v: Self::Error) -> Self;
}

impl<T> Try for Option<T> {
    type Ok = T;
    type Error = ();

    fn into_result(self) -> Result<T, ()> {
        self.ok_or(())
    }

    fn from_ok(v: T) -> Self {
        Some(v)
    }

    fn from_error(_: ()) -> Self {
        None
    }
}

impl<T, E> Try for Result<T, E> {
    type Ok = T;
    type Error = E;

    fn into_result(self) -> Result<T, E> {
        self
    }

    fn from_ok(v: T) -> Self {
        Ok(v)
    }

    fn from_error(v: E) -> Self {
        Err(v)
    }
}
