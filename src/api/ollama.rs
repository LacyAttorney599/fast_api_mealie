//! Appel à Ollama (`/api/generate`, réponse JSON non streamée), via le
//! reverse proxy (`/ollama/…`).

use serde_json::{Value, json};

use crate::api::error::ApiError;
use crate::api::http::{self, send_with_json, with_deadline};
use crate::api::config;
use crate::py;

/// Champ `response` d'une réponse `/api/generate`, décodé comme JSON.
pub fn parse_response(body: &Value) -> Result<Value, ApiError> {
    let Value::String(raw) = py::at(body, "response")? else {
        return Err(ApiError::Data(
            "the JSON object must be str, bytes or bytearray".to_owned(),
        ));
    };
    serde_json::from_str(raw).map_err(|err| ApiError::Json(err.to_string()))
}

/// Envoie `prompt` au modèle configuré et renvoie le JSON qu'il produit.
pub async fn generate_json(prompt: &str, options: Value, timeout_secs: u32) -> Result<Value, ApiError> {
    let model = config::ollama_model().await;
    let (request, _deadline) = with_deadline(http::post("/ollama/api/generate"), timeout_secs)?;
    let body = send_with_json(
        request,
        &json!({
            "model": model,
            "prompt": prompt,
            "format": "json",
            "stream": false,
            "options": options,
        }),
    )
    .await
    .map_err(|err| match err {
        ApiError::Transport(message) if message.contains("abort") => {
            ApiError::Transport(format!("Ollama n'a pas répondu en {timeout_secs} s"))
        }
        other => other,
    })?;
    parse_response(&body)
}
