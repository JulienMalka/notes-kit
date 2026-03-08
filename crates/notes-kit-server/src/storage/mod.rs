mod local;
#[cfg(feature = "s3")]
mod s3;

pub use local::LocalStorageBackend;
pub use notes_kit_core::traits::StorageBackend;
#[cfg(feature = "s3")]
pub use self::s3::S3StorageBackend;
