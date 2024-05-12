use std::fmt::Debug;
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VoxelifyError {
    #[error("Failed to serialize glTF")]
    SerializationError(#[from] gltf::json::Error),
    #[error("File size exceeds binary glTF limit")]
    SizeError(#[from] TryFromIntError),
}
