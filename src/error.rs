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

    #[error("Invalid name '{0}': names must contain only letters, digits, and underscores, and cannot start with a digit")]
    InvalidName(String),

    #[error("Missing argument: {0}")]
    MissingArgument(String),

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

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("Invalid snapshot: {0}")]
    InvalidSnapshot(String),

    #[error("FTS not enabled on collection '{0}'. Use `lodge alter {0} --fts \"field1,field2\"` to enable.")]
    FtsNotEnabled(String),

    #[error("FTS error: {0}")]
    Fts(String),

    #[error(
        "Field '{field}' in collection '{collection}' has wrong type (expected {expected_type})"
    )]
    WrongFieldType {
        field: String,
        collection: String,
        expected_type: String,
    },

    #[error("Field '{field}' not found in collection '{collection}'")]
    FieldNotFound { field: String, collection: String },

    #[error("Cannot modify protected field '{0}' (id, created_at, updated_at are auto-managed)")]
    ProtectedField(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid setting: {0}")]
    InvalidSetting(String),

    #[error("SQL error: {0}")]
    Sql(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, LodgeError>;
