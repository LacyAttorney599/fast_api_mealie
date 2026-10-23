//! Réglages fournis par le reverse proxy (`/config.json`, généré à partir des
//! variables d'environnement du conteneur).

use std::cell::RefCell;

use serde_json::Value;

use crate::api::http::{get, send_json};

const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5:3b";

thread_local! {
    static OLLAMA_MODEL: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Modèle Ollama (variable OLLAMA_MODEL) ; `qwen2.5:3b` si le réglage est
/// absent, par exemple avec `trunk serve` seul.
pub async fn ollama_model() -> String {
    if let Some(model) = OLLAMA_MODEL.with(|model| model.borrow().clone()) {
        return model;
    }
    let configured = send_json(get("/config.json"))
        .await
        .ok()
        .and_then(|config| config.get("ollamaModel").and_then(Value::as_str).map(str::to_owned))
        .filter(|model| !model.is_empty());
    let model = configured.unwrap_or_else(|| DEFAULT_OLLAMA_MODEL.to_owned());
    OLLAMA_MODEL.with(|cached| *cached.borrow_mut() = Some(model.clone()));
    model
}
