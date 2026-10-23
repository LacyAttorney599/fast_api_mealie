//! `localStorage` du navigateur et horloge. Hors navigateur (tests natifs),
//! un stockage en mémoire et l'horloge système les remplacent.

#[cfg(target_arch = "wasm32")]
mod platform {
    fn local_storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn get(key: &str) -> Option<String> {
        local_storage()?.get_item(key).ok()?
    }

    pub fn set(key: &str, value: &str) -> Result<(), String> {
        local_storage()
            .ok_or_else(|| "localStorage indisponible".to_owned())?
            .set_item(key, value)
            .map_err(|_| "écriture impossible dans le localStorage (quota atteint ?)".to_owned())
    }

    pub fn now_ms() -> f64 {
        js_sys::Date::now()
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod platform {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    thread_local! {
        static ITEMS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    }

    pub fn get(key: &str) -> Option<String> {
        ITEMS.with(|items| items.borrow().get(key).cloned())
    }

    pub fn set(key: &str, value: &str) -> Result<(), String> {
        ITEMS.with(|items| items.borrow_mut().insert(key.to_owned(), value.to_owned()));
        Ok(())
    }

    pub fn now_ms() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |elapsed| elapsed.as_secs_f64() * 1000.0)
    }
}

pub use platform::{get, now_ms, set};
