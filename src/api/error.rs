use std::fmt;

/// Erreur d'un appel à Mealie ou Ollama. Le texte reprend celui des exceptions
/// de l'ancien backend Python : l'import groupé l'affiche tel quel.
#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// Réponse hors 2xx (`httpx.HTTPStatusError`).
    Status {
        status: u16,
        url: String,
        location: Option<String>,
    },
    /// Échec réseau : serveur injoignable, délai dépassé…
    Transport(String),
    /// Corps de réponse qui n'est pas du JSON (`json.JSONDecodeError`).
    Json(String),
    /// Données d'une forme inattendue (`KeyError`, `TypeError`, `ValidationError`…).
    Data(String),
    /// Erreur métier signalée volontairement (`ValueError`), à montrer telle quelle.
    Invalid(String),
    /// Autres échecs (lecture de la photo, OCR, stockage du navigateur).
    Internal(String),
}

impl ApiError {
    /// Message destiné à l'utilisateur quand l'erreur en porte un (l'ancien
    /// `detail` des 400 du BFF), sinon `None`.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Invalid(message) | Self::Json(message) => Some(message),
            _ => None,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status {
                status,
                url,
                location,
            } => {
                let error_type = match status / 100 {
                    1 => "Informational response",
                    3 => "Redirect response",
                    4 => "Client error",
                    5 => "Server error",
                    _ => "Invalid status code",
                };
                let reason = http::StatusCode::from_u16(*status)
                    .ok()
                    .and_then(|status| status.canonical_reason())
                    .unwrap_or("");
                writeln!(f, "{error_type} '{status} {reason}' for url '{url}'")?;
                if let Some(location) = location {
                    writeln!(f, "Redirect location: '{location}'")?;
                }
                write!(
                    f,
                    "For more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/{status}"
                )
            }
            Self::Transport(message)
            | Self::Json(message)
            | Self::Data(message)
            | Self::Invalid(message)
            | Self::Internal(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_error_text_matches_httpx() {
        let err = ApiError::Status {
            status: 422,
            url: "http://192.168.1.250:8000/api/recipes".into(),
            location: None,
        };
        assert_eq!(
            err.to_string(),
            "Client error '422 Unprocessable Entity' for url 'http://192.168.1.250:8000/api/recipes'\n\
             For more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/422"
        );
    }

    #[test]
    fn redirect_error_text_includes_the_location() {
        let err = ApiError::Status {
            status: 307,
            url: "http://nas/api/recipes/".into(),
            location: Some("http://nas/api/recipes".into()),
        };
        assert_eq!(
            err.to_string(),
            "Redirect response '307 Temporary Redirect' for url 'http://nas/api/recipes/'\n\
             Redirect location: 'http://nas/api/recipes'\n\
             For more information check: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status/307"
        );
    }
}
