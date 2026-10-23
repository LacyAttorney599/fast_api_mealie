//! Écran « Importer une recette » : import d'une photo (OCR + IA, puis
//! formulaire d'ajout pré-rempli) ou import groupé d'un JSON de recettes.

use std::collections::BTreeSet;

use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_navigate;
use leptos_router::location::State;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, HtmlInputElement};

use crate::api::{self, DraftIngredient, RecipeDraft};
use crate::util::js;
use crate::util::parse_recipes_json::parse_recipes_json;

const SAMPLE_JSON: &str = r#"[
  {
    "nom": "Tarte aux pommes normande",
    "ingredients": [
      {"quantite":"6","unite":"—","aliment":"pommes"},
      {"quantite":"200","unite":"g","aliment":"farine"}
    ],
    "etapes": ["Éplucher les pommes...", "..."]
  },
  { "nom": "Gratin dauphinois", "...": "..." }
]"#;

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Photo,
    Json,
}

/// `n !== 1 && "s"` : le `false` ne s'affiche pas.
fn plural(n: usize) -> &'static str {
    if n != 1 { "s" } else { "" }
}

/// `[ing.quantite, ing.unite, ing.aliment].filter(Boolean).join(" ")` :
/// 0, `NaN` et les chaînes vides sont écartés.
fn ingredient_line(ing: &DraftIngredient) -> String {
    let quantite = ing
        .quantite
        .filter(|q| *q != 0.0 && !q.is_nan())
        .map(js::number_to_string);
    let unite = ing.unite.clone().filter(|u| !u.is_empty());
    let aliment = Some(ing.aliment.clone()).filter(|a| !a.is_empty());
    [quantite, unite, aliment]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Texte d'un fichier comme `FileReader.readAsText` : UTF-8 par défaut, mais
/// une marque d'ordre des octets UTF-16 est reconnue (`Blob.text()` ne lit
/// que l'UTF-8 ; PowerShell et le Bloc-notes peuvent enregistrer en UTF-16).
fn decode_text(bytes: &[u8]) -> String {
    fn utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
        let (pairs, odd_byte) = bytes.as_chunks::<2>();
        let mut text: String = char::decode_utf16(pairs.iter().map(|&pair| unit(pair)))
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        if !odd_byte.is_empty() {
            text.push(char::REPLACEMENT_CHARACTER);
        }
        text
    }
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// Premier fichier d'un `<input type="file">` après un `change`.
fn first_selected_file(ev: &leptos::ev::Event) -> Option<File> {
    ev.target()
        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
        .and_then(|input| input.files())
        .and_then(|files| files.get(0))
}

fn check_icon() -> impl IntoView {
    view! {
        <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#FFFFFF"
            stroke-width="3"
            stroke-linecap="round"
            stroke-linejoin="round"
        >
            <path d="M20 6 9 17l-5-5" />
        </svg>
    }
}

#[component]
pub fn Importer() -> impl IntoView {
    let tab = RwSignal::new(Tab::Photo);
    // Ne recrée le panneau que si l'onglet change vraiment (re-clic sur
    // l'onglet actif : rien ne bouge, comme en React).
    let is_photo_tab = Memo::new(move |_| tab.get() == Tab::Photo);

    // Import photo
    let file_name = RwSignal::new(None::<String>);
    let drag_over = RwSignal::new(false);
    let file_input_ref = NodeRef::<leptos::html::Input>::new();
    let navigate = use_navigate();
    let pending_draft = crate::app::use_pending_draft();

    let import_mutation = Action::new_local({
        let navigate = navigate.clone();
        move |file: &File| {
            let file = file.clone();
            let navigate = navigate.clone();
            async move {
                let result = api::import_photo(&file).await;
                if let Ok(draft) = &result {
                    // Ancien `navigate("/ajouter", { state: draft })` : le brouillon
                    // passe par `PendingDraft` et reste aussi dans l'entrée
                    // d'historique, pour un rechargement ou un retour sur /ajouter.
                    pending_draft.0.set(Some(draft.clone()));
                    let state = serde_json::to_string(draft)
                        .ok()
                        .map(|json| JsValue::from_str(&json));
                    navigate(
                        "/ajouter",
                        NavigateOptions {
                            state: State::new(state),
                            ..Default::default()
                        },
                    );
                }
                result
            }
        }
    });
    let import_pending = import_mutation.pending();
    let import_value = import_mutation.value();
    // `isError` de React Query : remis à faux dès qu'un nouvel import démarre.
    let import_is_error = Memo::new(move |_| {
        !import_pending.get() && import_value.with(|value| matches!(value, Some(Err(_))))
    });
    // `(isPending || fileName) && !isError` : un nom vide compte comme absent.
    let show_progress = Memo::new(move |_| {
        let has_file_name =
            file_name.with(|name| name.as_deref().is_some_and(|name| !name.is_empty()));
        (import_pending.get() || has_file_name) && !import_is_error.get()
    });

    let handle_file = move |file: File| {
        file_name.set(Some(file.name()));
        import_mutation.dispatch_local(file);
    };

    let handle_drop = move |ev: leptos::ev::DragEvent| {
        ev.prevent_default();
        drag_over.set(false);
        let file = ev
            .data_transfer()
            .and_then(|transfer| transfer.files())
            .and_then(|files| files.get(0));
        if let Some(file) = file {
            handle_file(file);
        }
    };

    // Import JSON
    let json_text = RwSignal::new(String::new());
    let selected = RwSignal::new(BTreeSet::<usize>::new());
    let expanded = RwSignal::new(None::<usize>);
    let json_file_input_ref = NodeRef::<leptos::html::Input>::new();

    let parsed = Memo::new(move |_| json_text.with(|text| parse_recipes_json(text)));

    // `useEffect(..., [parsed.recipes])` : chaque nouveau texte produit une
    // nouvelle analyse, donc toutes les recettes sont re-sélectionnées et
    // l'aperçu refermé. Un texte identique ne change rien (React ignore un
    // `setState` à la même valeur).
    let set_json_text = move |text: String| {
        if json_text
            .try_with_untracked(|current| *current == text)
            .unwrap_or(true)
        {
            return;
        }
        json_text.set(text);
        let count = parsed.with_untracked(|parsed| parsed.recipes.len());
        selected.set((0..count).collect());
        expanded.set(None);
    };

    let toggle_selected = move |index: usize| {
        selected.update(|current| {
            if !current.remove(&index) {
                current.insert(index);
            }
        });
    };

    let load_json_file = move |file: File| {
        leptos::task::spawn_local(async move {
            if let Ok(buffer) = JsFuture::from(file.array_buffer()).await {
                set_json_text(decode_text(&js_sys::Uint8Array::new(&buffer).to_vec()));
            }
        });
    };

    let bulk_mutation = Action::new_local(|drafts: &Vec<RecipeDraft>| {
        let drafts = drafts.clone();
        async move { api::create_recipes_bulk(&drafts).await }
    });
    let bulk_pending = bulk_mutation.pending();
    let results = bulk_mutation.value();

    let selected_recipes = move || {
        let chosen = selected.get_untracked();
        parsed.with_untracked(|parsed| {
            parsed
                .recipes
                .iter()
                .enumerate()
                .filter(|(index, _)| chosen.contains(index))
                .map(|(_, recipe)| recipe.clone())
                .collect::<Vec<_>>()
        })
    };

    let photo_panel = move || {
        view! {
            <div class="importer-photoPanel">
                {move || {
                    (!import_pending.get())
                        .then(|| {
                            view! {
                                <div
                                    class=move || {
                                        if drag_over.get() {
                                            "importer-dropzone importer-dropzoneActive"
                                        } else {
                                            "importer-dropzone"
                                        }
                                    }
                                    on:click=move |_| {
                                        if let Some(input) = file_input_ref.get() {
                                            input.click();
                                        }
                                    }
                                    on:dragover=move |ev: leptos::ev::DragEvent| {
                                        ev.prevent_default();
                                        if !drag_over.get_untracked() {
                                            drag_over.set(true);
                                        }
                                    }
                                    on:dragleave=move |_| {
                                        if drag_over.get_untracked() {
                                            drag_over.set(false);
                                        }
                                    }
                                    on:drop=handle_drop
                                >
                                    <input
                                        node_ref=file_input_ref
                                        type="file"
                                        accept="image/*"
                                        capture="environment"
                                        hidden
                                        on:change=move |ev| {
                                            if let Some(file) = first_selected_file(&ev) {
                                                handle_file(file);
                                            }
                                        }
                                    />
                                    <svg
                                        width="32"
                                        height="32"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="#C1592F"
                                        stroke-width="1.8"
                                    >
                                        <rect x="3" y="3" width="18" height="18" rx="3" />
                                        <circle cx="9" cy="9" r="2" />
                                        <path d="m21 15-5-5L5 21" />
                                    </svg>
                                    <p class="importer-dropzoneTitle">
                                        "Glissez une photo, ou cliquez pour choisir un fichier"
                                    </p>
                                    <p class="importer-dropzoneHint">
                                        "JPG, PNG — recette manuscrite ou imprimée"
                                    </p>
                                </div>
                            }
                        })
                }}

                {move || {
                    import_is_error
                        .get()
                        .then(|| {
                            view! {
                                <p class="importer-errorText">
                                    "Échec du traitement de la photo. Réessayez, ou vérifiez que le serveur IA (Ollama) est joignable."
                                </p>
                            }
                        })
                }}

                {move || {
                    show_progress
                        .get()
                        .then(|| {
                            view! {
                                <div class="importer-progress">
                                    <p class="importer-progressLabel">
                                        "Traitement — " {move || file_name.get().unwrap_or_default()}
                                    </p>
                                    <div class="importer-progressSteps">
                                        <div class="importer-progressStep">
                                            <span class="importer-stepDoneCircle">{check_icon()}</span>
                                            <span class="importer-stepDoneLabel">"Photo reçue"</span>
                                        </div>
                                        <div class="importer-progressLineDone"></div>
                                        <div class="importer-progressStep">
                                            <span class=move || {
                                                if import_pending.get() {
                                                    "importer-stepActiveCircle"
                                                } else {
                                                    "importer-stepDoneCircle"
                                                }
                                            }>
                                                {move || {
                                                    if import_pending.get() {
                                                        Either::Left("2")
                                                    } else {
                                                        Either::Right(check_icon())
                                                    }
                                                }}
                                            </span>
                                            <span class=move || {
                                                if import_pending.get() {
                                                    "importer-stepActiveLabel"
                                                } else {
                                                    "importer-stepDoneLabel"
                                                }
                                            }>"Lecture OCR & structuration IA…"</span>
                                        </div>
                                    </div>
                                </div>
                            }
                        })
                }}
            </div>
        }
    };

    let detected_rows = move || {
        let recipes = parsed.with(|parsed| parsed.recipes.clone());
        recipes
            .into_iter()
            .enumerate()
            .map(|(index, recipe)| {
                let is_expanded = Memo::new(move |_| expanded.get() == Some(index));
                let ingredient_count = recipe.ingredients.len();
                let step_count = recipe.etapes.len();
                let preview_lines: Vec<String> =
                    recipe.ingredients.iter().take(6).map(ingredient_line).collect();
                view! {
                    <div>
                        <div class="importer-detectedRow">
                            <input
                                type="checkbox"
                                prop:checked=move || selected.with(|selected| selected.contains(&index))
                                on:change=move |_| toggle_selected(index)
                                class="importer-checkbox"
                            />
                            <span class="importer-detectedName">{recipe.nom}</span>
                            <button
                                type="button"
                                class="importer-detectedView"
                                on:click=move |_| {
                                    expanded
                                        .update(|current| {
                                            *current = if *current == Some(index) { None } else { Some(index) };
                                        })
                                }
                            >
                                {move || if is_expanded.get() { "Masquer" } else { "Voir" }}
                            </button>
                        </div>
                        {move || {
                            is_expanded
                                .get()
                                .then(|| {
                                    let items = preview_lines
                                        .iter()
                                        .map(|line| view! { <li>{line.clone()}</li> })
                                        .collect_view();
                                    view! {
                                        <div class="importer-preview">
                                            <span>
                                                {format!(
                                                    "{ingredient_count} ingrédient{}",
                                                    plural(ingredient_count),
                                                )}
                                            </span>
                                            <ul class="importer-previewList">
                                                {items}
                                                {(ingredient_count > 6).then(|| view! { <li>"…"</li> })}
                                            </ul>
                                            <span>{format!("{step_count} étape{}", plural(step_count))}</span>
                                        </div>
                                    }
                                })
                        }}
                    </div>
                }
            })
            .collect_view()
    };

    let json_footer = move || match results.get() {
        Some(results) => {
            let success_count = results.iter().filter(|result| result.success).count();
            let total = results.len();
            let errors = results
                .into_iter()
                .filter(|result| !result.success)
                .map(|result| {
                    view! {
                        <span class="importer-errorText">
                            {format!("{} : {}", result.nom, result.error.unwrap_or_default())}
                        </span>
                    }
                })
                .collect_view();
            let navigate = navigate.clone();
            Either::Left(view! {
                <div class="importer-resultsSummary">
                    <span>
                        {format!(
                            "{success_count}/{total} recette{} importée{}.",
                            plural(total),
                            plural(success_count),
                        )}
                    </span>
                    {errors}
                    <button
                        class="importer-importButton"
                        on:click=move |_| navigate("/", Default::default())
                    >
                        "Voir la liste des recettes"
                    </button>
                </div>
            })
        }
        None => Either::Right(view! {
            <button
                class="importer-importButton"
                disabled=move || selected.with(BTreeSet::is_empty) || bulk_pending.get()
                on:click=move |_| {
                    bulk_mutation.dispatch(selected_recipes());
                }
            >
                {move || {
                    if bulk_pending.get() {
                        "Import en cours…".to_owned()
                    } else {
                        format!("Importer les recettes sélectionnées ({})", selected.with(BTreeSet::len))
                    }
                }}
            </button>
        }),
    };

    let json_panel = move || {
        view! {
            <div class="importer-jsonPanel">
                <div class="importer-jsonColumns">
                    <div class="importer-jsonInputColumn">
                        <label class="importer-label">"Coller le JSON généré par Claude"</label>
                        <textarea
                            class="importer-textarea"
                            prop:value=move || json_text.get()
                            on:input=move |ev| set_json_text(event_target_value(&ev))
                            placeholder=SAMPLE_JSON
                        ></textarea>
                        <input
                            node_ref=json_file_input_ref
                            type="file"
                            accept="application/json,.json"
                            hidden
                            on:change=move |ev| {
                                if let Some(file) = first_selected_file(&ev) {
                                    load_json_file(file);
                                }
                            }
                        />
                        <button
                            class="importer-loadFileButton"
                            on:click=move |_| {
                                if let Some(input) = json_file_input_ref.get() {
                                    input.click();
                                }
                            }
                        >
                            "Charger un fichier .json"
                        </button>
                        {move || {
                            parsed
                                .with(|parsed| parsed.error)
                                .map(|error| view! { <p class="importer-errorText">{error}</p> })
                        }}
                    </div>
                    <div class="importer-detectedColumn">
                        <label class="importer-label">
                            {move || {
                                let count = parsed.with(|parsed| parsed.recipes.len());
                                format!("{count} recette{} détectée{}", plural(count), plural(count))
                            }}
                        </label>
                        <div class="importer-detectedList">{detected_rows}</div>
                    </div>
                </div>
                <div class="importer-jsonFooter">{json_footer.clone()}</div>
            </div>
        }
    };

    view! {
        <div>
            <h1 class="importer-title">"Importer une recette"</h1>
            <p class="importer-subtitle">
                "Depuis une photo unique, ou depuis un JSON généré par Claude à partir d'un livre PDF."
            </p>
        </div>

        <div class="importer-tabs">
            <button
                class=move || if tab.get() == Tab::Photo { "importer-tabActive" } else { "importer-tab" }
                on:click=move |_| tab.set(Tab::Photo)
            >
                "Import photo"
            </button>
            <button
                class=move || if tab.get() == Tab::Json { "importer-tabActive" } else { "importer-tab" }
                on:click=move |_| tab.set(Tab::Json)
            >
                "Import JSON (livre PDF)"
            </button>
        </div>

        {move || if is_photo_tab.get() { Either::Left(photo_panel()) } else { Either::Right(json_panel()) }}
    }
}

#[cfg(test)]
mod tests {
    use super::decode_text;

    #[test]
    fn decode_text_reads_utf8_with_or_without_bom() {
        assert_eq!(
            decode_text("[{\"nom\":\"Crêpes\"}]".as_bytes()),
            "[{\"nom\":\"Crêpes\"}]"
        );
        assert_eq!(decode_text(b"\xEF\xBB\xBF[]"), "[]");
        assert_eq!(decode_text(b"caf\xE9"), "caf\u{FFFD}");
    }

    #[test]
    fn decode_text_reads_utf16_with_bom() {
        let le: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("Crêpe".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode_text(&le), "Crêpe");
        let be: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain("Crêpe".encode_utf16().flat_map(u16::to_be_bytes))
            .collect();
        assert_eq!(decode_text(&be), "Crêpe");
        assert_eq!(decode_text(&[0xFF, 0xFE, 0x41, 0x00, 0x42]), "A\u{FFFD}");
    }
}
