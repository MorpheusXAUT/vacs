pub mod flight_information_region;
pub mod network;
pub mod position;
pub mod profile;
pub mod station;

use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum CoverageError {
    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Io(#[from] IoError),

    #[error(transparent)]
    Structure(#[from] StructureError),
}

#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    #[error("invalid format for `{field}`: {reason}")]
    InvalidFormat {
        field: String,
        value: String,
        reason: String,
    },

    #[error("invalid value for `{field}`: {reason} (got `{value}`)")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    #[error("value for `{field}` is out of range: {min}..={max} (got `{value}`)")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },

    #[error("`{field}` must not be empty")]
    Empty { field: String },

    #[error("referenced {field} `{ref_id}` does not exist")]
    MissingReference { field: String, ref_id: String },

    #[error("{0}")]
    Custom(String),
}

#[derive(Debug, Clone, Error)]
pub enum IoError {
    #[error("failed to read `{path}`: {reason}")]
    Read {
        path: std::path::PathBuf,
        reason: String,
    },

    #[error("failed to parse `{path}`: {reason}")]
    Parse {
        path: std::path::PathBuf,
        reason: String,
    },

    #[error("failed to read directory entry: {0}")]
    ReadEntry(String),
}

#[derive(Debug, Clone, Error)]
pub enum StructureError {
    #[error("duplicate {entity} `{id}`")]
    Duplicate { entity: String, id: String },

    #[error("station `{0}` has no coverage")]
    EmptyCoverage(String),

    #[error("failed to load {entity} from `{id}`: {reason}")]
    Load {
        entity: String,
        id: String,
        reason: String,
    },
}

pub trait Validator {
    fn validate(&self) -> Result<(), CoverageError>;
}

pub trait ReferenceValidator<T> {
    fn validate_references(&self, entities: &HashSet<&T>) -> Result<(), CoverageError>;
}
