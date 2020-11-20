mod core;
mod executor;
mod iter;
mod misc;
mod std;

pub use self::core::{
    Consumer, Driver, Executor, ExecutorCallback, Folder, FromParallelIterator,
    IndexedParallelIterator, IndexedProducer, IndexedProducerCallback, IntoParallelIterator,
    IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelDrainFull, ParallelDrainRange,
    ParallelExtend, ParallelIterator, Producer, ProducerCallback, Reducer, Setup,
    WithIndexedProducer, WithProducer, WithSetup,
};
#[cfg(feature = "default-executor")]
pub use self::executor::DefaultExecutor;
#[cfg(feature = "rayon-executor")]
pub use self::executor::RayonExecutor;
#[cfg(feature = "sequential-executor")]
pub use self::executor::SequentialExecutor;
#[cfg(feature = "tokio-executor")]
pub use self::executor::TokioExecutor;
pub use executor::{IndexedSplitter, Splitter};
