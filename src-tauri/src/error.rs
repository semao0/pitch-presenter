//! Application error type. Messages are user-facing and follow the UI language.

use std::fmt;
use std::path::Path;

use serde::{Serialize, Serializer};

use crate::i18n::tr;

#[derive(Debug)]
pub enum Error {
    Io {
        path: String,
        source: std::io::Error,
    },
    Notes {
        file: String,
        message: String,
    },
    Pdf(String),
    NoLibreOffice,
    Conversion(String),
    NoPdfium,
    EmptyDeck,
    NoSession,
    Tauri(tauri::Error),
}

impl Error {
    pub fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Io { path, source } => format!("{path}: {source}"),
            Self::Notes { file, message } => {
                tr!("Ошибка в {file}: {message}", "Error in {file}: {message}")
            }
            Self::Pdf(e) => tr!("Не удалось открыть PDF: {e}", "Cannot open the PDF: {e}"),
            Self::NoLibreOffice => tr!(
                "Для PPTX, PPT, ODP и Key нужен LibreOffice. Установите его или экспортируйте презентацию в PDF.",
                "PPTX, PPT, ODP and Key need LibreOffice. Install it or export the presentation to PDF."
            ),
            Self::Conversion(e) => tr!(
                "LibreOffice не смог сконвертировать презентацию: {e}",
                "LibreOffice could not convert the presentation: {e}"
            ),
            Self::NoPdfium => tr!(
                "Не найдена библиотека PDFium. Переустановите приложение.",
                "The PDFium library is missing. Please reinstall the app."
            ),
            Self::EmptyDeck => tr!(
                "В презентации нет слайдов",
                "The presentation has no slides"
            ),
            Self::NoSession => tr!("Сначала откройте презентацию", "Open a presentation first"),
            Self::Tauri(e) => e.to_string(),
        };
        f.write_str(&message)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Tauri(e) => Some(e),
            _ => None,
        }
    }
}

impl From<tauri::Error> for Error {
    fn from(e: tauri::Error) -> Self {
        Self::Tauri(e)
    }
}

/// Commands return errors to the frontend as plain message strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
