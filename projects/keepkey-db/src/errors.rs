use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Migration error: {0}")]
    Migration(String),
    
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    
    #[error("Setup step invalid: expected {expected}, got {actual}")]
    InvalidSetupStep { expected: u8, actual: u8 },
    
    #[error("Device setup not complete: {0}")]
    SetupNotComplete(String),
    
    #[error("Database connection error: {0}")]
    Connection(String),
    
    #[error("Transaction error: {0}")]
    Transaction(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>; 