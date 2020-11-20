mod misc;
#[cfg(feature = "rayon-executor")]
mod rayon;
mod simple;
#[cfg(feature = "tokio-executor")]
mod tokio;

#[cfg(feature = "rayon-executor")]
pub use self::rayon::Rayon as RayonExecutor;
pub use self::simple::Simple as SimpleExecutor;
#[cfg(feature = "tokio-executor")]
pub use self::tokio::Tokio as TokioExecutor;

#[cfg(feature = "rayon-executor")]
pub type DefaultExecutor = RayonExecutor;

#[cfg(all(not(feature = "rayon-executor"), feature = "tokio-executor"))]
pub type DefaultExecutor = TokioExecutor;

#[cfg(all(not(feature = "rayon-executor"), not(feature = "tokio-executor")))]
pub type DefaultExecutor = SimpleExecutor;
