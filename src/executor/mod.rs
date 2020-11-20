mod misc;
#[cfg(feature = "rayon-executor")]
mod rayon;
#[cfg(feature = "sequential-executor")]
mod sequential;
#[cfg(feature = "tokio-executor")]
mod tokio;

pub use misc::{IndexedSplitter, Splitter};

#[cfg(feature = "rayon-executor")]
pub use self::rayon::Rayon as RayonExecutor;
#[cfg(feature = "sequential-executor")]
pub use self::sequential::Sequential as SequentialExecutor;
#[cfg(feature = "tokio-executor")]
pub use self::tokio::Tokio as TokioExecutor;

#[cfg(feature = "rayon-executor")]
pub type DefaultExecutor = RayonExecutor;

#[cfg(all(feature = "tokio-executor", not(feature = "rayon-executor")))]
pub type DefaultExecutor = TokioExecutor;

#[cfg(all(
    feature = "sequential-executor",
    not(feature = "rayon-executor"),
    not(feature = "tokio-executor")
))]
pub type DefaultExecutor = SequentialExecutor;
