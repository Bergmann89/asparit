mod simple;
#[cfg(feature = "tokio-executor")]
mod tokio;

#[cfg(feature = "tokio-executor")]
pub use self::tokio::Tokio as TokioExecutor;
pub use simple::Simple as SimpleExecutor;

#[cfg(feature = "tokio-executor")]
pub type DefaultExecutor = TokioExecutor;

#[cfg(not(feature = "tokio-executor"))]
pub type DefaultExecutor = SimpleExecutor;
