use thiserror::Error;
use std::error::Error as StdError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    
    #[error("Content error: {0}")]
    Content(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid content: {0}")]
    InvalidContent(String),
    
    #[error("Server error: {0}")]
    Server(String),
}

impl From<Box<dyn StdError>> for AppError {
    fn from(err: Box<dyn StdError>) -> Self {
        AppError::Server(err.to_string())
    }
}

impl actix_web::ResponseError for AppError {
    fn error_response(&self) -> actix_web::HttpResponse {
        use actix_web::http::StatusCode;
        
        let status = match self {
            AppError::FileNotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidContent(_) => StatusCode::BAD_REQUEST,
            AppError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        
        tracing::error!("Error: {}", self);
        
        actix_web::HttpResponse::build(status)
            .content_type("text/plain")
            .body(self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
