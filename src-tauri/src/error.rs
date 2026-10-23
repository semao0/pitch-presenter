//! Application error type. Messages are user-facing (Russian) because they are
//! shown as-is in the UI.

use std::path::Path;

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },

    #[error("Ошибка в {file}: {message}")]
    Notes { file: String, message: String },

    #[error("Не удалось открыть PDF: {0}")]
    Pdf(String),

    #[error(
        "Для PPTX, PPT, ODP и Key нужен LibreOffice. Установите его или экспортируйте презентацию в PDF."
    )]
    NoLibreOffice,

    #[error("LibreOffice не смог сконвертировать презентацию: {0}")]
    Conversion(String),

    #[error("Не найдена библиотека PDFium. Переустановите приложение.")]
    NoPdfium,

    #[error("В презентации нет слайдов")]
    EmptyDeck,

    #[error("Сначала откройте презентацию")]
    NoSession,

    #[error(transparent)]
    Tauri(#[from] tauri::Error),
}

impl Error {
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

/// Commands return errors to the frontend as plain message strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
