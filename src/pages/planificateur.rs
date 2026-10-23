//! Planificateur : déjeuners et dîners de la semaine (ex-`Planificateur.tsx`).

use std::collections::HashSet;

use leptos::html;
use leptos::prelude::*;

use crate::api::{self, MealPlanEntry};
use crate::app::{bump, use_refresh};
use crate::util::dates;

/// Créneau en cours de remplissage (date ISO + "lunch" ou "dinner").
#[derive(Debug, Clone, PartialEq)]
struct Slot {
    date: String,
    meal_type: &'static str,
}

fn find_entry<'a>(
    entries: &'a [MealPlanEntry],
    date: &str,
    meal_type: &str,
) -> Option<&'a MealPlanEntry> {
    entries
        .iter()
        .find(|entry| entry.date == date && entry.entry_type == meal_type)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RecipeOption {
    slug: String,
    name: String,
    tag: Option<String>,
    image_url: Option<String>,
    color: String,
    seasonal: bool,
}

#[component]
fn RecipePickerModal(
    #[prop(into)] recipes: Signal<Vec<RecipeOption>>,
    #[prop(into)] slot_label: Signal<String>,
    on_select: Callback<String>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let filtered = Memo::new(move |_| {
        let query = query.get().to_lowercase();
        recipes.with(|recipes| {
            recipes
                .iter()
                .filter(|recipe| recipe.name.to_lowercase().contains(&query))
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    // autoFocus
    let search_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        if let Some(input) = search_input.get() {
            let _ = input.focus();
        }
    });

    // Liste à clés (comme `key={recipe.slug}` en React) : la recherche garde les
    // éléments déjà affichés au lieu de tout recréer à chaque frappe. La clé est
    // la recette entière, pour qu'un badge « de saison » arrivé après coup
    // redessine l'élément.
    let item = move |recipe: RecipeOption| {
        let slug = recipe.slug;
        // `recipe.image_url ? … : …` : une chaîne vide compte comme absente.
        let image_url = recipe.image_url.filter(|url| !url.is_empty());
        let background = image_url.is_none().then_some(recipe.color);
        view! {
            <button class="planificateur-modalItem" on:click=move |_| on_select.run(slug.clone())>
                <div class="planificateur-modalItemThumb" style:background=background>
                    {image_url
                        .map(|src| {
                            view! { <img src=src alt="" class="planificateur-modalItemThumbImg" /> }
                        })}
                </div>
                <div class="planificateur-modalItemInfo">
                    <span class="planificateur-modalItemName">{recipe.name}</span>
                    <span class="planificateur-modalItemMeta">
                        {recipe
                            .seasonal
                            .then(|| {
                                view! { <span class="planificateur-modalSeasonBadge">"🌱 De saison"</span> }
                            })}
                        {recipe.tag.unwrap_or_else(|| "Sans catégorie".to_string())}
                    </span>
                </div>
            </button>
        }
    };

    let on_escape = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            on_cancel.run(());
        }
    };

    view! {
        <div class="planificateur-modalOverlay" on:click=move |_| on_cancel.run(()) on:keydown=on_escape>
            <div class="planificateur-modal" on:click=|ev| ev.stop_propagation()>
                <div class="planificateur-modalHeader">
                    <div>
                        <span class="planificateur-modalTitle">"Choisir une recette"</span>
                        <span class="planificateur-modalSlot">{move || slot_label.get()}</span>
                    </div>
                    <button class="planificateur-modalClose" on:click=move |_| on_cancel.run(())>
                        "×"
                    </button>
                </div>
                <div class="planificateur-modalSearchWrapper">
                    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#8A7F6E" stroke-width="2">
                        <circle cx="11" cy="11" r="8" />
                        <path d="m21 21-4.3-4.3" />
                    </svg>
                    <input
                        node_ref=search_input
                        class="planificateur-modalSearchInput"
                        placeholder="Rechercher une recette…"
                        prop:value=move || query.get()
                        on:input=move |ev| query.set(event_target_value(&ev))
                        on:keydown=on_escape
                    />
                </div>
                <div class="planificateur-modalList">
                    <For each=move || filtered.get() key=|recipe| recipe.clone() children=item />

                    {move || {
                        filtered
                            .with(Vec::is_empty)
                            .then(|| {
                                view! {
                                    <span class="planificateur-modalEmpty">
                                        "Aucune recette ne correspond à votre recherche."
                                    </span>
                                }
                            })
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn Planificateur() -> impl IntoView {
    let refresh = use_refresh();
    let week_start = RwSignal::new(dates::get_monday(dates::today()));
    let adding_slot = RwSignal::new(None::<Slot>);

    let days = Memo::new(move |_| dates::get_week_days(week_start.get()));
    let iso_start = Memo::new(move |_| dates::to_iso_date(week_start.get()));
    let iso_end = Memo::new(move |_| days.with(|days| days[6].iso.clone()));

    let entries = LocalResource::new(move || {
        let (start, end) = (iso_start.get(), iso_end.get());
        refresh.mealplan.track();
        async move { api::fetch_meal_plan(&start, &end).await }
    });
    // Comme React Query, un rechargement en échec garde le dernier planning
    // obtenu (sinon tous les créneaux redeviendraient « + Ajouter »). Les
    // créneaux cherchent par date : la liste d'une autre semaine n'y apparaît pas.
    let shown_entries = Memo::new(move |previous: Option<&Vec<MealPlanEntry>>| {
        entries.with(|result| match result {
            Some(Ok(list)) => list.clone(),
            _ => previous.cloned().unwrap_or_default(),
        })
    });

    let recipes = LocalResource::new(move || {
        refresh.recipes.track();
        async move { api::fetch_recipes("", "").await }
    });

    // Calculé à la demande (pas sur l'écran Liste) : il faut le détail de
    // chaque recette pour savoir si elle est de saison.
    let seasonal_recipes = LocalResource::new(api::fetch_seasonal_recipes);

    let current_season = api::fetch_current_season();

    let sorted_recipes = Memo::new(move |_| {
        let seasonal_slugs: HashSet<String> = seasonal_recipes.with(|result| match result {
            Some(Ok(list)) => list
                .iter()
                .filter(|recipe| recipe.in_season)
                .map(|recipe| recipe.slug.clone())
                .collect(),
            _ => HashSet::new(),
        });
        recipes.with(|result| match result {
            Some(Ok(list)) => {
                let mut sorted: Vec<_> = list.iter().collect();
                // Tri stable : les recettes de saison d'abord, ordre conservé sinon.
                sorted.sort_by_key(|recipe| !seasonal_slugs.contains(&recipe.slug));
                Some(
                    sorted
                        .into_iter()
                        .map(|recipe| RecipeOption {
                            slug: recipe.slug.clone(),
                            name: recipe.name.clone(),
                            tag: recipe.tag.clone(),
                            image_url: recipe.image_url.clone(),
                            color: recipe.color.clone(),
                            seasonal: seasonal_slugs.contains(&recipe.slug),
                        })
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        })
    });

    let create_mutation = Action::new_local(
        move |(date, meal_type, recipe_slug): &(String, &'static str, String)| {
            let (date, meal_type, recipe_slug) = (date.clone(), *meal_type, recipe_slug.clone());
            async move {
                let result = api::create_meal_plan_entry(&date, meal_type, &recipe_slug).await;
                if result.is_ok() {
                    bump(refresh.mealplan);
                    adding_slot.set(None);
                }
                result
            }
        },
    );

    let delete_mutation = Action::new_local(move |id: &i64| {
        let id = *id;
        async move {
            let result = api::delete_meal_plan_entry(id).await;
            if result.is_ok() {
                bump(refresh.mealplan);
            }
            result
        }
    });

    let generate_ai_mutation = Action::new_local(move |(start, end): &(String, String)| {
        let (start, end) = (start.clone(), end.clone());
        async move {
            let result = api::generate_ai_meal_plan(&start, &end).await;
            if result.is_ok() {
                bump(refresh.mealplan);
                // Le `onSuccess` de React renvoyait la promesse d'`invalidateQueries` :
                // la mutation restait « en cours » jusqu'au rechargement du planning.
                // Un tick laisse la ressource démarrer son rechargement, puis on
                // attend qu'il se termine.
                leptos::task::tick().await;
                if !entries.is_disposed() {
                    let _ = entries.await;
                }
            }
            result
        }
    });
    let generate_ai_pending = generate_ai_mutation.pending();
    // `isError` de React Query : faux pendant une nouvelle tentative.
    let generate_ai_is_error = move || {
        !generate_ai_pending.get()
            && generate_ai_mutation
                .value()
                .with(|value| matches!(value, Some(Err(_))))
    };

    let render_slot = move |date: String, meal_type: &'static str, dot_color: &'static str| {
        move || {
            let entry = shown_entries.with(|list| find_entry(list, &date, meal_type).cloned());

            if let Some(entry) = entry {
                let id = entry.id;
                let name_view = match entry.recipe_slug.filter(|slug| !slug.is_empty()) {
                    Some(slug) => view! {
                        <a href=format!("/recette/{slug}") class="planificateur-slotName">
                            {entry.name}
                        </a>
                    }
                    .into_any(),
                    None => view! { <span class="planificateur-slotName">{entry.name}</span> }
                        .into_any(),
                };
                return view! {
                    <div class="planificateur-filledSlot">
                        <div class="planificateur-filledSlotHeader">
                            <span class="planificateur-dot" style:background=dot_color></span>
                            <button
                                aria-label="Retirer du planning"
                                class="planificateur-removeButton"
                                on:click=move |_| {
                                    delete_mutation.dispatch(id);
                                }
                            >
                                "×"
                            </button>
                        </div>
                        {name_view}
                    </div>
                }
                .into_any();
            }

            let slot_date = date.clone();
            let is_adding = move || {
                adding_slot.with(|slot| {
                    slot.as_ref()
                        .is_some_and(|slot| slot.date == slot_date && slot.meal_type == meal_type)
                })
            };
            let click_date = date.clone();
            view! {
                <button
                    class=move || {
                        if is_adding() { "planificateur-emptySlotActive" } else { "planificateur-emptySlot" }
                    }
                    on:click=move |_| {
                        adding_slot
                            .set(
                                Some(Slot {
                                    date: click_date.clone(),
                                    meal_type,
                                }),
                            )
                    }
                >
                    "+ Ajouter"
                </button>
            }
            .into_any()
        }
    };

    let adding_slot_label = Signal::derive(move || {
        adding_slot.with(|slot| match slot {
            Some(slot) => {
                let meal = if slot.meal_type == "lunch" {
                    "Déjeuner"
                } else {
                    "Dîner"
                };
                let day = days.with(|days| {
                    days.iter()
                        .find(|day| day.iso == slot.date)
                        .map(|day| day.label.clone())
                        .unwrap_or_else(|| slot.date.clone())
                });
                format!("{meal} · {day}")
            }
            None => String::new(),
        })
    });

    let on_select = Callback::new(move |recipe_slug: String| {
        if let Some(slot) = adding_slot.get_untracked() {
            create_mutation.dispatch((slot.date, slot.meal_type, recipe_slug));
        }
    });
    let on_cancel = Callback::new(move |()| adding_slot.set(None));

    view! {
        <Show when=move || adding_slot.with(Option::is_some) && sorted_recipes.with(Option::is_some)>
            <RecipePickerModal
                recipes=Signal::derive(move || sorted_recipes.get().unwrap_or_default())
                slot_label=adding_slot_label
                on_select=on_select
                on_cancel=on_cancel
            />
        </Show>
        <div class="planificateur-header">
            <div>
                <h1 class="planificateur-title">"Planificateur"</h1>
                <p class="planificateur-subtitle">"Organisez déjeuners et dîners de la semaine."</p>
                <p class="planificateur-seasonHint">
                    "🌱 De saison ce mois-ci : " {current_season.produce.join(", ")}
                </p>
            </div>
            <div class="planificateur-weekNav">
                <button
                    aria-label="Semaine précédente"
                    class="planificateur-navButton"
                    on:click=move |_| week_start.update(|current| *current = dates::add_days(*current, -7))
                >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#4A4238" stroke-width="2.2">
                        <path d="m15 18-6-6 6-6" />
                    </svg>
                </button>
                <span class="planificateur-weekLabel">{move || dates::format_week_range(week_start.get())}</span>
                <button
                    aria-label="Semaine suivante"
                    class="planificateur-navButton"
                    on:click=move |_| week_start.update(|current| *current = dates::add_days(*current, 7))
                >
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#4A4238" stroke-width="2.2">
                        <path d="m9 18 6-6-6-6" />
                    </svg>
                </button>
            </div>
        </div>

        <div class="planificateur-grid">
            <div />
            {move || {
                days.get()
                    .into_iter()
                    .map(|day| view! { <div class="planificateur-dayLabel">{day.label}</div> })
                    .collect_view()
            }}

            <div class="planificateur-rowLabel">"Déjeuner"</div>
            {move || {
                days.get()
                    .into_iter()
                    .map(|day| view! { <div>{render_slot(day.iso, "lunch", "#C1592F")}</div> })
                    .collect_view()
            }}

            <div class="planificateur-rowLabel">"Dîner"</div>
            {move || {
                days.get()
                    .into_iter()
                    .map(|day| view! { <div>{render_slot(day.iso, "dinner", "#6B7A5E")}</div> })
                    .collect_view()
            }}
        </div>

        {move || {
            generate_ai_is_error()
                .then(|| {
                    view! {
                        <p class="planificateur-errorText">
                            "Échec de la génération IA. Vérifiez que le serveur Ollama est joignable."
                        </p>
                    }
                })
        }}

        <div class="planificateur-footer">
            <button
                class="planificateur-generateAiButton"
                disabled=move || generate_ai_pending.get()
                on:click=move |_| {
                    generate_ai_mutation.dispatch((iso_start.get_untracked(), iso_end.get_untracked()));
                }
            >
                {move || if generate_ai_pending.get() { "Génération en cours…" } else { "✨ Générer avec l'IA" }}
            </button>
            <a
                href=move || format!("/courses?start={}&end={}", iso_start.get(), iso_end.get())
                class="planificateur-generateButton"
            >
                "Générer la liste de courses"
            </a>
        </div>
    }
}
