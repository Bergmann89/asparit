mod sequential;
#[cfg(feature = "tokio-executor")]
mod tokio;

pub use sequential::Sequential as SequentialExecutor;
#[cfg(feature = "tokio-executor")]
pub use self::tokio::Tokio as TokioExecutor;

#[cfg(feature = "tokio-executor")]
pub type DefaultExecutor = TokioExecutor;

#[cfg(not(feature = "tokio-executor"))]
pub type DefaultExecutor = SequentialExecutor;
