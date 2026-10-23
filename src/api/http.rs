//! Requêtes vers Mealie (`/api/…`) et Ollama (`/ollama/…`). Le même reverse
//! proxy sert l'application et ces deux API : les chemins sont relatifs, et
//! c'est lui qui ajoute le token Mealie (jamais exposé au navigateur).
//!
//! Sémantique reprise d'httpx : `raise_for_status()` puis `response.json()`.

use gloo_net::http::{Request, RequestBuilder, Response};
use gloo_timers::callback::Timeout;
use serde::Serialize;
use serde_json::Value;
use web_sys::AbortController;

use crate::api::error::ApiError;

pub fn get(path: &str) -> RequestBuilder {
    Request::get(path)
}

pub fn post(path: &str) -> RequestBuilder {
    Request::post(path)
}

pub fn put(path: &str) -> RequestBuilder {
    Request::put(path)
}

pub fn delete(path: &str) -> RequestBuilder {
    Request::delete(path)
}

fn transport(err: gloo_net::Error) -> ApiError {
    ApiError::Transport(err.to_string())
}

/// `raise_for_status()` : un statut hors 2xx devient une erreur.
fn check(response: Response) -> Result<Response, ApiError> {
    if response.ok() {
        return Ok(response);
    }
    let status = response.status();
    let location = matches!(status, 301 | 302 | 303 | 307 | 308)
        .then(|| response.headers().get("location"))
        .flatten();
    Err(ApiError::Status {
        status,
        url: response.url(),
        location,
    })
}

async fn json(response: Response) -> Result<Value, ApiError> {
    let text = response.text().await.map_err(transport)?;
    serde_json::from_str(&text).map_err(|err| ApiError::Json(err.to_string()))
}

/// Envoie une requête sans corps.
pub async fn send(request: RequestBuilder) -> Result<Response, ApiError> {
    check(request.send().await.map_err(transport)?)
}

/// Envoie une requête avec un corps JSON.
pub async fn send_with(request: RequestBuilder, body: &impl Serialize) -> Result<Response, ApiError> {
    let request = request.json(body).map_err(transport)?;
    check(request.send().await.map_err(transport)?)
}

/// `send` puis `response.json()`.
pub async fn send_json(request: RequestBuilder) -> Result<Value, ApiError> {
    json(send(request).await?).await
}

/// `send_with` puis `response.json()`.
pub async fn send_with_json(request: RequestBuilder, body: &impl Serialize) -> Result<Value, ApiError> {
    json(send_with(request, body).await?).await
}

/// Annule la requête au-delà de `seconds` (le `timeout` des clients httpx).
/// À garder en vie jusqu'à la réponse : sa destruction désarme le minuteur.
pub struct Deadline {
    _timer: Timeout,
}

pub fn with_deadline(request: RequestBuilder, seconds: u32) -> Result<(RequestBuilder, Deadline), ApiError> {
    let controller = AbortController::new().map_err(|_| ApiError::Internal("AbortController indisponible".to_owned()))?;
    let request = request.abort_signal(Some(&controller.signal()));
    let timer = Timeout::new(seconds * 1000, move || controller.abort());
    Ok((request, Deadline { _timer: timer }))
}
