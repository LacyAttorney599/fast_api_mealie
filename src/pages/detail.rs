//! Écran Détail d'une recette (ex-`frontend/src/pages/Detail.tsx`) : route
//! `/recette/:slug`.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::api::{self, ApiError, Cookbook, RecipeDetail};
use crate::app::{bump, use_refresh};
use crate::util::{dates, js};

/// `formatQuantity` : arrondi au centième, virgule décimale, puis l'unité.
fn format_quantity(quantity: f64, unit: Option<&str>) -> String {
    let rounded = js::math_round(quantity * 100.0) / 100.0;
    let qty_str = js::number_to_string(rounded).replacen('.', ",", 1);
    match unit.filter(|unit| !unit.is_empty()) {
        Some(unit) => format!("{qty_str} {unit}"),
        None => qty_str,
    }
}

/// Nombre « vrai » au sens de JavaScript (ni absent, ni `0`, ni `NaN`).
fn truthy_number(n: Option<f64>) -> Option<f64> {
    n.filter(|n| *n != 0.0 && !n.is_nan())
}

/// Texte de l'erreur d'`addRecipeToCookbook` : le `detail` que renvoyait
/// l'ancien backend, sinon le message générique avec le statut HTTP (500
/// pour une erreur sans statut, ce que répondait le backend dans ce cas).
fn cookbook_error_message(err: &ApiError) -> String {
    match err.detail() {
        Some(detail) => detail.to_owned(),
        None => {
            let status = match err {
                ApiError::Status { status, .. } => *status,
                _ => 500,
            };
            format!("Échec de l'ajout au livre ({status})")
        }
    }
}

fn back_link() -> impl IntoView {
    view! {
        <a href="/" class="detail-back">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#6B5F51" stroke-width="2.2">
                <path d="m15 18-6-6 6-6" />
            </svg>
            "Retour aux recettes"
        </a>
    }
}

#[component]
pub fn Detail() -> impl IntoView {
    let params = use_params_map();
    let slug = Memo::new(move |_| params.with(|p| p.get("slug").unwrap_or_default()));
    let refresh = use_refresh();

    let planning_open = RwSignal::new(false);
    let planning_date = RwSignal::new(dates::to_iso_date(dates::today()));
    let planning_meal = RwSignal::new(String::from("dinner"));

    let cookbook_panel_open = RwSignal::new(false);
    let selected_cookbook = RwSignal::new(String::new());

    let custom_servings = RwSignal::new(None::<f64>);
    Effect::new(move |_| {
        slug.track();
        custom_servings.set(None);
    });

    // Le résultat est étiqueté par le slug demandé : pendant le chargement
    // d'une autre recette, la ressource garde l'ancienne valeur, alors que
    // React affichait « Chargement… ». `None` : requête désactivée (slug vide).
    let recipe = LocalResource::new(move || {
        let slug = slug.get();
        async move {
            let result = if slug.is_empty() {
                None
            } else {
                Some(api::fetch_recipe(&slug).await)
            };
            (slug, result)
        }
    });

    // `data` de la requête des livres : dernière liste reçue, conservée quand
    // le panneau se ferme (cache de React Query). Chargée seulement quand le
    // panneau est ouvert (`enabled: cookbookPanelOpen`).
    let cookbooks = RwSignal::new(None::<Vec<Cookbook>>);
    let _cookbooks_query = LocalResource::new(move || {
        let enabled = cookbook_panel_open.get();
        refresh.cookbooks.track();
        async move {
            // `try_` : la page a pu être quittée pendant la requête, et lire un
            // signal détruit fait planter l'application.
            if enabled
                && let Ok(list) = api::fetch_cookbooks().await
                && cookbooks
                    .try_with_untracked(|current| current.as_ref() != Some(&list))
                    .unwrap_or(false)
            {
                cookbooks.set(Some(list));
            }
        }
    });

    let plan_mutation = Action::new_local(
        move |(date, meal, recipe_slug): &(String, String, String)| {
            let (date, meal, recipe_slug) = (date.clone(), meal.clone(), recipe_slug.clone());
            async move {
                let result = api::create_meal_plan_entry(&date, &meal, &recipe_slug)
                    .await
                    .map(|_| ());
                if result.is_ok() {
                    bump(refresh.mealplan);
                    planning_open.set(false);
                }
                result
            }
        },
    );

    let add_to_cookbook_mutation =
        Action::new_local(move |(cookbook_slug, recipe_slug): &(String, String)| {
            let (cookbook_slug, recipe_slug) = (cookbook_slug.clone(), recipe_slug.clone());
            async move {
                let result = api::add_recipe_to_cookbook(&cookbook_slug, &recipe_slug).await;
                if result.is_ok() {
                    cookbook_panel_open.set(false);
                }
                result
            }
        });

    // `isError` de React Query : remis à faux dès qu'une nouvelle mutation part.
    let plan_error = move || {
        !plan_mutation.pending().get()
            && plan_mutation
                .value()
                .with(|value| matches!(value, Some(Err(_))))
    };
    let cookbook_error = move || {
        if add_to_cookbook_mutation.pending().get() {
            return None;
        }
        add_to_cookbook_mutation.value().with(|value| match value {
            Some(Err(err)) => Some(cookbook_error_message(err)),
            _ => None,
        })
    };

    let planning_panel = move || {
        planning_open.get().then(|| {
            view! {
                <div class="detail-planningPanel">
                    <input
                        type="date"
                        class="detail-planningDate"
                        // Valeur posée une fois à l'ouverture : la réécrire à chaque
                        // saisie (comme un `prop:value` réactif) effacerait le jour,
                        // le mois et l'année dès qu'un seul segment est vidé.
                        prop:value=planning_date.get_untracked()
                        on:input=move |ev| planning_date.set(event_target_value(&ev))
                    />
                    // `prop:selected` sur les options : la valeur du <select> est posée
                    // avant l'ajout de ses options, elle ne suffit pas au premier rendu.
                    <select
                        class="detail-planningMeal"
                        prop:value=move || planning_meal.get()
                        on:change=move |ev| planning_meal.set(event_target_value(&ev))
                    >
                        <option value="lunch" prop:selected=move || planning_meal.with(|meal| meal == "lunch")>
                            "Déjeuner"
                        </option>
                        <option value="dinner" prop:selected=move || planning_meal.with(|meal| meal == "dinner")>
                            "Dîner"
                        </option>
                    </select>
                    <button
                        class="detail-planningConfirm"
                        disabled=move || plan_mutation.pending().get()
                        on:click=move |_| {
                            plan_mutation
                                .dispatch((
                                    planning_date.get_untracked(),
                                    planning_meal.get_untracked(),
                                    slug.get_untracked(),
                                ));
                        }
                    >
                        {move || if plan_mutation.pending().get() { "Ajout…" } else { "Confirmer" }}
                    </button>
                    {move || plan_error().then(|| view! { <span class="detail-planningError">"Échec de l'ajout"</span> })}
                </div>
            }
        })
    };

    let cookbook_options = move || {
        cookbooks
            .get()
            .unwrap_or_default()
            .into_iter()
            .filter(|cookbook| cookbook.manual)
            .map(|cookbook| {
                let value = cookbook.slug.clone();
                view! {
                    <option
                        value=cookbook.slug
                        prop:selected=move || selected_cookbook.with(|selected| *selected == value)
                    >
                        {cookbook.name}
                    </option>
                }
            })
            .collect_view()
    };

    let cookbook_panel = move || {
        cookbook_panel_open.get().then(|| {
            view! {
                <div class="detail-planningPanel">
                    <select
                        class="detail-planningMeal"
                        prop:value=move || selected_cookbook.get()
                        on:change=move |ev| selected_cookbook.set(event_target_value(&ev))
                    >
                        <option
                            value=""
                            disabled=true
                            prop:selected=move || selected_cookbook.with(String::is_empty)
                        >
                            "Choisir un livre…"
                        </option>
                        {cookbook_options}
                    </select>
                    <button
                        class="detail-planningConfirm"
                        disabled=move || {
                            selected_cookbook.with(String::is_empty) || add_to_cookbook_mutation.pending().get()
                        }
                        on:click=move |_| {
                            add_to_cookbook_mutation
                                .dispatch((selected_cookbook.get_untracked(), slug.get_untracked()));
                        }
                    >
                        {move || if add_to_cookbook_mutation.pending().get() { "Ajout…" } else { "Ajouter" }}
                    </button>
                    {move || {
                        cookbook_error().map(|message| view! { <span class="detail-planningError">{message}</span> })
                    }}
                </div>
            }
        })
    };

    // Rendu d'une recette chargée. Construit une seule fois par recette : seuls
    // les parts, quantités et panneaux sont réactifs (les cases à cocher des
    // ingrédients gardent leur état quand on change le nombre de parts).
    let recipe_view = move |recipe: RecipeDetail, recipe_slug: String| {
        let base_servings = recipe.servings;
        let display_servings = move || custom_servings.get().or(base_servings);
        let servings_scale = Memo::new(move |_| {
            match (
                truthy_number(base_servings),
                truthy_number(display_servings()),
            ) {
                (Some(base), Some(display)) => display / base,
                _ => 1.0,
            }
        });
        let time = recipe.time.filter(|time| !time.is_empty());
        let has_base_servings = truthy_number(base_servings).is_some();

        let image = match recipe.image_url.filter(|url| !url.is_empty()) {
            Some(url) => view! { <img class="detail-photo" src=url alt="" /> }.into_any(),
            None => view! {
                <svg
                    class="detail-imageIcon"
                    width="56"
                    height="56"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="#2B2420"
                    stroke-width="1.2"
                    opacity="0.3"
                >
                    <path d="M12 2a7 7 0 0 0-7 7c0 3 2 5 2 8h10c0-3 2-5 2-8a7 7 0 0 0-7-7z" />
                    <path d="M9 21h6" />
                </svg>
            }
            .into_any(),
        };

        let meta_time = time.clone().map(|time| view! { <span>{time}</span> });
        let meta_dot = (time.is_some() && has_base_servings)
            .then(|| view! { <span class="detail-metaDot">"·"</span> });
        let servings_control = has_base_servings.then(|| {
            view! {
                <div class="detail-servingsControl">
                    <button
                        aria-label="Moins de parts"
                        class="detail-servingsButton"
                        on:click=move |_| {
                            let current = custom_servings.get_untracked().or(base_servings).unwrap_or_default();
                            custom_servings.set(Some(f64::max(1.0, current - 1.0)));
                        }
                    >
                        "−"
                    </button>
                    <span class="detail-servingsValue">
                        {move || display_servings().map(js::number_to_string)} " personne"
                        {move || if display_servings() != Some(1.0) { "s" } else { "" }}
                    </span>
                    <button
                        aria-label="Plus de parts"
                        class="detail-servingsButton"
                        on:click=move |_| {
                            let current = custom_servings.get_untracked().or(base_servings).unwrap_or_default();
                            custom_servings.set(Some(current + 1.0));
                        }
                    >
                        "+"
                    </button>
                </div>
            }
        });

        let tags = (!recipe.tags.is_empty() || recipe.in_season).then(|| {
            let season_badge = recipe
                .in_season
                .then(|| view! { <span class="detail-seasonBadge">"🌱 De saison"</span> });
            let tag_list = recipe
                .tags
                .into_iter()
                .map(|tag| view! { <span class="detail-tag">{tag}</span> })
                .collect_view();
            view! { <div class="detail-tags">{season_badge} {tag_list}</div> }
        });

        let description = (!recipe.description.is_empty())
            .then(|| view! { <p class="detail-description">{recipe.description}</p> });

        let ingredient_rows = recipe
            .ingredients
            .into_iter()
            .map(|ingredient| {
                let unit = ingredient.unit;
                let qty = ingredient.quantity.map(|quantity| {
                    view! {
                        <span class="detail-ingredientQty">
                            {move || format_quantity(quantity * servings_scale.get(), unit.as_deref())}
                        </span>
                    }
                });
                view! {
                    <div class="detail-ingredientRow">
                        <input type="checkbox" class="detail-checkbox" />
                        {qty}
                        <span>{ingredient.food}</span>
                    </div>
                }
            })
            .collect_view();

        let step_rows = recipe
            .steps
            .into_iter()
            .enumerate()
            .map(|(i, text)| {
                view! {
                    <div class="detail-stepRow">
                        <span class="detail-stepNumber">{i + 1}</span>
                        <p class="detail-stepText">{text}</p>
                    </div>
                }
            })
            .collect_view();

        view! {
            {back_link()}

            <div class="detail-hero">
                <div class="detail-image">{image}</div>
                <div class="detail-heroInfo">
                    <div class="detail-heroTop">
                        <h1 class="detail-title">{recipe.name}</h1>
                        <div class="detail-actions">
                            <button
                                class="detail-primaryButton"
                                on:click=move |_| planning_open.update(|open| *open = !*open)
                            >
                                "Ajouter au planning"
                            </button>
                            <button
                                class="detail-secondaryButton"
                                on:click=move |_| cookbook_panel_open.update(|open| *open = !*open)
                            >
                                "Ajouter à un livre"
                            </button>
                            <a href=format!("/recette/{recipe_slug}/modifier") class="detail-secondaryButton">
                                "Modifier"
                            </a>
                        </div>
                    </div>

                    {planning_panel}

                    {cookbook_panel}
                    <div class="detail-meta">{meta_time} {meta_dot} {servings_control}</div>
                    {tags}
                    {description}
                </div>
            </div>

            <div class="detail-body">
                <div class="detail-ingredients">
                    <h2 class="detail-sectionTitle">"Ingrédients"</h2>
                    <div class="detail-ingredientList">{ingredient_rows}</div>
                </div>
                <div class="detail-steps">
                    <h2 class="detail-sectionTitle">"Préparation"</h2>
                    <div class="detail-stepList">{step_rows}</div>
                </div>
            </div>
        }
    };

    move || {
        let current_slug = slug.get();
        match recipe.get() {
            Some((fetched_slug, result)) if fetched_slug == current_slug => match result {
                Some(Ok(recipe)) => recipe_view(recipe, fetched_slug).into_any(),
                _ => {
                    view! {
                        {back_link()}
                        <p class="detail-status">"Impossible de charger cette recette depuis Mealie."</p>
                    }
                    .into_any()
                }
            },
            _ => view! { <p class="detail-status">"Chargement…"</p> }.into_any(),
        }
    }
}
