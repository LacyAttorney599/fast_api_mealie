//! Formulaire de création (`/ajouter`) et de modification
//! (`/recette/:slug/modifier`) d'une recette.

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api::{self, ApiError, DraftIngredient, RecipeDraft};
use crate::util::js;

/// Ligne d'ingrédient telle que saisie (texte libre dans les trois champs).
#[derive(Debug, Clone, Default, PartialEq)]
struct Ingredient {
    qty: String,
    unit: String,
    food: String,
}

/// Valeurs du formulaire.
struct FormValues {
    nom: String,
    ingredients: Vec<Ingredient>,
    steps: Vec<String>,
}

/// `fromDraft` : valeurs initiales du formulaire, avec au moins une ligne
/// d'ingrédient et une étape vides.
fn from_draft(draft: Option<&RecipeDraft>) -> FormValues {
    let Some(draft) = draft else {
        return FormValues {
            nom: String::new(),
            ingredients: vec![Ingredient::default()],
            steps: vec![String::new()],
        };
    };
    FormValues {
        nom: draft.nom.clone(),
        ingredients: if draft.ingredients.is_empty() {
            vec![Ingredient::default()]
        } else {
            draft
                .ingredients
                .iter()
                .map(|ing| Ingredient {
                    qty: ing.quantite.map(js::number_to_string).unwrap_or_default(),
                    unit: ing.unite.clone().unwrap_or_default(),
                    food: ing.aliment.clone(),
                })
                .collect()
        },
        steps: if draft.etapes.is_empty() {
            vec![String::new()]
        } else {
            draft.etapes.clone()
        },
    }
}

/// `Number(qty.replace(",", "."))` tel que `JSON.stringify` l'envoyait :
/// `NaN` et `±Infinity` devenaient `null`, `-0` devenait `0`.
fn parse_quantity(qty: &str) -> Option<f64> {
    if js::trim(qty).is_empty() {
        return None;
    }
    let value = js::string_to_number(&qty.replacen(',', ".", 1));
    if !value.is_finite() {
        return None;
    }
    Some(if value == 0.0 { 0.0 } else { value })
}

/// `handleSave` : brouillon envoyé à Mealie (ingrédients sans aliment et
/// étapes vides ignorés).
fn build_draft(nom: String, ingredients: &[Ingredient], steps: &[String]) -> RecipeDraft {
    RecipeDraft {
        nom,
        ingredients: ingredients
            .iter()
            .filter(|ing| !js::trim(&ing.food).is_empty())
            .map(|ing| {
                let unit = js::trim(&ing.unit);
                DraftIngredient {
                    quantite: parse_quantity(&ing.qty),
                    unite: (!unit.is_empty()).then(|| unit.to_owned()),
                    aliment: js::trim(&ing.food).to_owned(),
                }
            })
            .collect(),
        etapes: steps
            .iter()
            .map(|step| js::trim(step))
            .filter(|step| !step.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}

/// Brouillon de l'import photo gardé dans l'entrée d'historique de /ajouter
/// (l'ancien `location.state`) : il survit à un rechargement et revient avec
/// un retour arrière.
fn history_draft() -> Option<RecipeDraft> {
    let state = window().history().ok()?.state().ok()?;
    serde_json::from_str(&state.as_string()?).ok()
}

/// Résultat du chargement de la recette à modifier, avec le slug auquel il
/// correspond (`None` en mode création : rien n'est chargé).
type DraftLoad = (Option<String>, Option<Result<RecipeDraft, ApiError>>);

#[component]
pub fn Ajouter() -> impl IntoView {
    let navigate = use_navigate();
    let params = use_params_map();
    let slug = Memo::new(move |_| {
        params
            .with(|p| p.get("slug"))
            .filter(|slug| !slug.is_empty())
    });
    let is_edit_mode = Memo::new(move |_| slug.with(Option::is_some));

    let existing_draft = LocalResource::new(move || {
        let slug = slug.get();
        async move {
            let result = match &slug {
                Some(slug) => Some(api::fetch_recipe_draft(slug).await),
                None => None,
            };
            let load: DraftLoad = (slug, result);
            load
        }
    });
    // Chargement en cours tant qu'aucun résultat ne correspond au slug courant.
    let is_loading_draft = Memo::new(move |_| match slug.get() {
        Some(current) => existing_draft.with(
            |loaded| !matches!(loaded, Some((Some(loaded_slug), _)) if *loaded_slug == current),
        ),
        None => false,
    });

    // Brouillon transmis par l'import photo (l'ancien `location.state`) :
    // celui que l'import vient de déposer, sinon celui de l'entrée d'historique.
    let initial_draft = if slug.get_untracked().is_none() {
        let pending = crate::app::use_pending_draft();
        match pending.0.get_untracked() {
            Some(draft) => {
                pending.0.set(None);
                Some(draft)
            }
            None => history_draft(),
        }
    } else {
        None
    };
    let initial = from_draft(initial_draft.as_ref());

    let nom = RwSignal::new(initial.nom);
    let ingredients = RwSignal::new(initial.ingredients);
    let steps = RwSignal::new(initial.steps);

    Effect::new(move |_| {
        if let Some((Some(loaded_slug), Some(Ok(draft)))) = existing_draft.get()
            && slug.get_untracked().as_ref() == Some(&loaded_slug)
        {
            let mapped = from_draft(Some(&draft));
            nom.set(mapped.nom);
            ingredients.set(mapped.ingredients);
            steps.set(mapped.steps);
        }
    });

    let save_mutation = Action::new_local({
        let navigate = navigate.clone();
        move |(slug, draft): &(Option<String>, RecipeDraft)| {
            let slug = slug.clone();
            let draft = draft.clone();
            let navigate = navigate.clone();
            async move {
                let result = match &slug {
                    Some(slug) => api::update_recipe(slug, &draft).await,
                    None => api::create_recipe(&draft).await,
                };
                if let Ok(result_slug) = &result {
                    navigate(&format!("/recette/{result_slug}"), Default::default());
                }
                result
            }
        }
    });
    let is_pending = save_mutation.pending();
    // Comme React Query, l'erreur précédente disparaît pendant un nouvel essai.
    let is_error = move || {
        !is_pending.get()
            && save_mutation
                .value()
                .with(|value| matches!(value, Some(Err(_))))
    };

    let navigate = StoredValue::new_local(navigate);
    let handle_cancel = move |_| {
        let path = match slug.get_untracked() {
            Some(slug) => format!("/recette/{slug}"),
            None => "/".to_owned(),
        };
        let navigate = navigate.get_value();
        navigate(&path, Default::default());
    };

    let handle_save = move |_| {
        let draft = ingredients.with_untracked(|ingredients| {
            steps.with_untracked(|steps| build_draft(nom.get_untracked(), ingredients, steps))
        });
        save_mutation.dispatch((slug.get_untracked(), draft));
    };

    let update_ingredient = move |index: usize, patch: &dyn Fn(&mut Ingredient)| {
        ingredients.update(|current| {
            if let Some(ing) = current.get_mut(index) {
                patch(ing);
            }
        });
    };
    let ingredient_field = move |index: usize, field: fn(&Ingredient) -> &String| {
        move || {
            ingredients.with(|current| {
                current
                    .get(index)
                    .map(|ing| field(ing).clone())
                    .unwrap_or_default()
            })
        }
    };

    view! {
        <Show
            when=move || !is_loading_draft.get()
            fallback=|| view! { <p class="ajouter-subtitle">"Chargement…"</p> }
        >
            <div>
                <h1 class="ajouter-title">
                    {move || {
                        if is_edit_mode.get() { "Modifier la recette" } else { "Nouvelle recette" }
                    }}
                </h1>
                <p class="ajouter-subtitle">
                    {move || {
                        if is_edit_mode.get() {
                            "Modifiez les informations et enregistrez."
                        } else {
                            "Saisissez les informations, ou importez-les depuis une photo ou un livre PDF."
                        }
                    }}
                </p>
            </div>

            <Show when=move || !is_edit_mode.get()>
                <div class="ajouter-tabs">
                    <span class="ajouter-tabActive">"Saisie manuelle"</span>
                    <a href="/importer" class="ajouter-tab">
                        "Import photo"
                    </a>
                    <a href="/importer" class="ajouter-tab">
                        "Import JSON (livre PDF)"
                    </a>
                </div>
            </Show>

            <div class="ajouter-form">
                <div class="ajouter-topRow">
                    <div class="ajouter-field">
                        <label class="ajouter-label">"Nom de la recette"</label>
                        <input
                            class="ajouter-input"
                            prop:value=move || nom.get()
                            on:input=move |ev| nom.set(event_target_value(&ev))
                        />
                    </div>
                    <Show when=move || !is_edit_mode.get()>
                        <div class="ajouter-field">
                            <label class="ajouter-label">"Importer depuis une URL"</label>
                            <div class="ajouter-urlRow">
                                // Champ et bouton sans effet, comme dans l'interface React.
                                <input class="ajouter-input" placeholder="https://…" />
                                <button class="ajouter-extractButton">"Extraire"</button>
                            </div>
                        </div>
                    </Show>
                </div>

                <div class="ajouter-columns">
                    <div class="ajouter-ingredientsColumn">
                        <div class="ajouter-columnHeader">
                            <label class="ajouter-label">"Ingrédients"</label>
                            <button
                                class="ajouter-addLink"
                                on:click=move |_| {
                                    ingredients.update(|current| current.push(Ingredient::default()))
                                }
                            >
                                "+ Ajouter un ingrédient"
                            </button>
                        </div>
                        <div class="ajouter-ingredientsHead">
                            <span>"QTÉ"</span>
                            <span>"UNITÉ"</span>
                            <span>"ALIMENT"</span>
                        </div>
                        // Lignes indexées par position (`key={i}` côté React).
                        <For
                            each=move || 0..ingredients.with(Vec::len)
                            key=|index| *index
                            children=move |index| {
                                view! {
                                    <div class="ajouter-ingredientRow">
                                        <input
                                            class="ajouter-smallInput"
                                            prop:value=ingredient_field(index, |ing| &ing.qty)
                                            on:input=move |ev| {
                                                let value = event_target_value(&ev);
                                                update_ingredient(index, &|ing| ing.qty.clone_from(&value));
                                            }
                                        />
                                        <input
                                            class="ajouter-smallInput"
                                            prop:value=ingredient_field(index, |ing| &ing.unit)
                                            on:input=move |ev| {
                                                let value = event_target_value(&ev);
                                                update_ingredient(index, &|ing| ing.unit.clone_from(&value));
                                            }
                                        />
                                        <input
                                            class="ajouter-smallInput"
                                            prop:value=ingredient_field(index, |ing| &ing.food)
                                            on:input=move |ev| {
                                                let value = event_target_value(&ev);
                                                update_ingredient(index, &|ing| ing.food.clone_from(&value));
                                            }
                                        />
                                    </div>
                                }
                            }
                        />
                    </div>

                    <div class="ajouter-stepsColumn">
                        <div class="ajouter-columnHeader">
                            <label class="ajouter-label">"Étapes"</label>
                            <button
                                class="ajouter-addLink"
                                on:click=move |_| steps.update(|current| current.push(String::new()))
                            >
                                "+ Ajouter une étape"
                            </button>
                        </div>
                        <For
                            each=move || 0..steps.with(Vec::len)
                            key=|index| *index
                            children=move |index| {
                                view! {
                                    <div class="ajouter-stepRow">
                                        <span class="ajouter-stepNumber">{index + 1}</span>
                                        <textarea
                                            class="ajouter-textarea"
                                            prop:value=move || {
                                                steps.with(|current| current.get(index).cloned().unwrap_or_default())
                                            }
                                            on:input=move |ev| {
                                                let text = event_target_value(&ev);
                                                steps.update(|current| {
                                                    if let Some(step) = current.get_mut(index) {
                                                        *step = text;
                                                    }
                                                });
                                            }
                                        ></textarea>
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>
            </div>

            <Show when=is_error>
                <p class="ajouter-error">"Échec de l'enregistrement vers Mealie."</p>
            </Show>

            <div class="ajouter-footer">
                <button class="ajouter-cancelButton" on:click=handle_cancel>
                    "Annuler"
                </button>
                <button
                    class="ajouter-saveButton"
                    disabled=move || is_pending.get()
                    on:click=handle_save
                >
                    {move || {
                        if is_pending.get() {
                            "Enregistrement…"
                        } else if is_edit_mode.get() {
                            "Enregistrer les modifications"
                        } else {
                            "Enregistrer la recette"
                        }
                    }}
                </button>
            </div>
        </Show>
    }
}
