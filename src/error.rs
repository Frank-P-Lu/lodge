use thiserror::Error;

#[derive(Error, Debug)]
pub enum LodgeError {
    #[error("Lodge already initialized in this directory")]
    AlreadyInitialized,

    #[error("No lodge database found. Run `lodge init` first.")]
    NotInitialized,

    #[error("Collection '{0}' already exists")]
    CollectionExists(String),

    #[error("Collection '{0}' not found")]
    CollectionNotFound(String),

    #[error("Invalid field type '{0}'. Valid types: text, int, real, bool, date, datetime")]
    InvalidFieldType(String),

    #[error("Invalid fields format: {0}")]
    InvalidFieldsFormat(String),

    #[error("Reserved name '{0}' cannot be used as a collection name")]
    ReservedName(String),

    #[error("Invalid value for field '{field}' (type {field_type}): {value}")]
    InvalidValue {
        field: String,
        field_type: String,
        value: String,
    },

    #[error("Record with id {0} not found")]
    RecordNotFound(i64),

    #[error("View '{0}' already exists")]
    ViewExists(String),

    #[error("View '{0}' not found")]
    ViewNotFound(String),

    #[error("Import error: {0}")]
    ImportError(String),

    #[error("SQL error: {0}")]
    Sql(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, LodgeError>;
