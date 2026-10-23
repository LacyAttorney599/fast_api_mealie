//! Écran principal : liste des recettes, filtres (de saison, tout, livres de
//! recettes), création d'un livre, favoris et pagination.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::api::{self, ApiError, Cookbook, RecipeCategory, RecipeSummary};
use crate::app::{bump, use_refresh};
use crate::util::js;

const PAGE_SIZE: usize = 50;

#[derive(Debug, Clone, PartialEq)]
enum Filter {
    All,
    Seasonal,
    Cookbook(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NewCookbookMode {
    Manual,
    Category,
}

/// Clé de la requête des recettes de React Query : `["recipes", search, selectedCookbook]`.
type RecipesKey = (String, Option<String>);

/// Clé d'une carte : son contenu, pour qu'une carte dont les données changent
/// soit redessinée. Le favori n'en fait pas partie : il est suivi dans la
/// carte, qui reste donc en place quand on le change (comme avec la clé
/// `slug` de React, le focus clavier est conservé).
type CardKey = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

fn card_key(recipe: &RecipeSummary) -> CardKey {
    (
        recipe.slug.clone(),
        recipe.name.clone(),
        recipe.time.clone(),
        recipe.tag.clone(),
        recipe.image_url.clone(),
        recipe.color.clone(),
    )
}

fn filter_class(active: bool) -> &'static str {
    if active {
        "liste-filterActive"
    } else {
        "liste-filter"
    }
}

#[component]
pub fn Liste() -> impl IntoView {
    let refresh = use_refresh();

    let search = RwSignal::new(String::new());
    let filter = RwSignal::new(Filter::Seasonal);
    let page = RwSignal::new(1_usize);
    let creating_cookbook = RwSignal::new(false);
    let new_cookbook_name = RwSignal::new(String::new());
    let new_cookbook_mode = RwSignal::new(NewCookbookMode::Manual);
    let new_cookbook_category_id = RwSignal::new(String::new());
    let new_cookbook_panel_ref = NodeRef::<html::Div>::new();
    let new_cookbook_input_ref = NodeRef::<html::Input>::new();

    let selected_cookbook = Memo::new(move |_| {
        filter.with(|filter| match filter {
            Filter::Cookbook(slug) => Some(slug.clone()),
            _ => None,
        })
    });
    let is_seasonal = Memo::new(move |_| filter.with(|filter| *filter == Filter::Seasonal));
    let is_all = Memo::new(move |_| filter.with(|filter| *filter == Filter::All));
    let is_category_mode = Memo::new(move |_| new_cookbook_mode.get() == NewCookbookMode::Category);

    // `useEffect(() => setPage(1), [search, filter])` : React compare par
    // identité, donc une chaîne identique ne change rien alors qu'un nouvel
    // objet `{ cookbook }` (même livre recliqué) revient à la page 1.
    let select_filter = move |next: Filter| {
        let changed = matches!(next, Filter::Cookbook(_))
            || filter.with_untracked(|current| *current != next);
        if changed {
            filter.set(next);
            page.set(1);
        }
    };

    let reset_new_cookbook_form = move || {
        creating_cookbook.set(false);
        new_cookbook_name.set(String::new());
        new_cookbook_mode.set(NewCookbookMode::Manual);
        new_cookbook_category_id.set(String::new());
    };

    // Un clic en dehors du panneau « Nouveau livre » l'abandonne ; l'écouteur
    // n'existe que tant que le panneau est ouvert.
    Effect::new(move |_| {
        if !creating_cookbook.get() {
            return;
        }
        let handle = window_event_listener(leptos::ev::mousedown, move |event| {
            let Some(panel) = new_cookbook_panel_ref.get_untracked() else {
                return;
            };
            let target = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::Node>().ok());
            if !panel.contains(target.as_ref()) {
                reset_new_cookbook_form();
            }
        });
        on_cleanup(move || handle.remove());
    });

    // `autoFocus` du nom du livre, à chaque ouverture du panneau.
    Effect::new(move |_| {
        if let Some(input) = new_cookbook_input_ref.get() {
            let _ = input.focus();
        }
    });

    let cookbooks_query = LocalResource::new(move || {
        refresh.cookbooks.track();
        api::fetch_cookbooks()
    });
    // Comme React Query, un rechargement en échec garde la liste déjà obtenue.
    let cookbooks = Memo::new(
        move |previous: Option<&Option<Vec<Cookbook>>>| match cookbooks_query.get() {
            Some(Ok(cookbooks)) => Some(cookbooks),
            _ => previous.cloned().flatten(),
        },
    );

    // `enabled: creatingCookbook` : chargées à chaque ouverture du panneau, la
    // dernière liste obtenue restant affichée en attendant (cache React Query).
    // Mise à jour seulement si la liste a changé, pour ne pas recréer les
    // options pendant qu'une catégorie est choisie.
    let categories = RwSignal::new(None::<Vec<RecipeCategory>>);
    let _categories_query = LocalResource::new(move || {
        let enabled = creating_cookbook.get();
        async move {
            // `try_` : la page a pu être quittée pendant la requête, et lire un
            // signal détruit fait planter l'application.
            if enabled
                && let Ok(list) = api::fetch_cookbook_categories().await
                && categories
                    .try_with_untracked(|current| current.as_ref() != Some(&list))
                    .unwrap_or(false)
            {
                categories.set(Some(list));
            }
        }
    });

    let create_cookbook = Action::new_local(move |_: &()| {
        let name = js::trim(&new_cookbook_name.get_untracked()).to_owned();
        let category_id = (new_cookbook_mode.get_untracked() == NewCookbookMode::Category)
            .then(|| new_cookbook_category_id.get_untracked());
        async move {
            let cookbook = api::create_cookbook(&name, category_id.as_deref()).await?;
            bump(refresh.cookbooks);
            filter.set(Filter::Cookbook(cookbook.slug));
            page.set(1);
            reset_new_cookbook_form();
            Ok::<(), ApiError>(())
        }
    });

    let remove_from_cookbook = Action::new_local(move |recipe_slug: &String| {
        let cookbook_slug = selected_cookbook.get_untracked().unwrap_or_default();
        let recipe_slug = recipe_slug.clone();
        async move {
            api::remove_recipe_from_cookbook(&cookbook_slug, &recipe_slug).await?;
            bump(refresh.recipes);
            Ok::<(), ApiError>(())
        }
    });

    let favorite = Action::new_local(move |(slug, favorite): &(String, bool)| {
        let (slug, favorite) = (slug.clone(), *favorite);
        async move {
            if favorite {
                api::add_favorite(&slug).await?;
            } else {
                api::remove_favorite(&slug).await?;
            }
            bump(refresh.recipes);
            Ok::<(), ApiError>(())
        }
    });

    let active_cookbook = Memo::new(move |_| {
        let selected = selected_cookbook.get()?;
        cookbooks.with(|cookbooks| {
            cookbooks
                .as_ref()?
                .iter()
                .find(|cookbook| cookbook.slug == selected)
                .cloned()
        })
    });
    let can_remove_from_book = Memo::new(move |_| {
        selected_cookbook.with(|selected| selected.as_deref().is_some_and(|slug| !slug.is_empty()))
            && active_cookbook
                .with(|cookbook| cookbook.as_ref().is_some_and(|cookbook| cookbook.manual))
    });

    // Requête par clé (recherche, livre) comme React Query : une clé jamais
    // chargée affiche « Chargement… », une clé déjà vue réaffiche aussitôt
    // ses recettes, et un rechargement (favori, retrait) garde l'affichage.
    let recipes_key = Memo::new(move |_| (search.get(), selected_cookbook.get()));
    let recipes_cache = RwSignal::new(HashMap::<RecipesKey, Arc<Vec<RecipeSummary>>>::new());
    let recipes_query = LocalResource::new(move || {
        let key = recipes_key.get();
        refresh.recipes.track();
        async move {
            let result = api::fetch_recipes(&key.0, key.1.as_deref().unwrap_or(""))
                .await
                .map(Arc::new);
            if let Ok(recipes) = &result {
                recipes_cache.update(|cache| {
                    cache.insert(key.clone(), Arc::clone(recipes));
                });
            }
            (key, result)
        }
    });
    let all_recipes = Memo::new(move |_| {
        recipes_key.with(|key| recipes_cache.with(|cache| cache.get(key).cloned()))
    });
    let is_error = Memo::new(move |_| {
        let key = recipes_key.get();
        matches!(recipes_query.get(), Some((loaded_key, Err(_))) if loaded_key == key)
    });
    let is_loading = Memo::new(move |_| all_recipes.with(Option::is_none) && !is_error.get());
    let favorite_slugs = Memo::new(move |_| {
        all_recipes.with(|recipes| {
            recipes
                .iter()
                .flat_map(|recipes| recipes.iter())
                .filter(|recipe| recipe.is_favorite)
                .map(|recipe| recipe.slug.clone())
                .collect::<HashSet<_>>()
        })
    });

    // Mis en cache par la couche API (une heure) : pas de rechargement ici.
    let seasonal_query = LocalResource::new(api::fetch_seasonal_recipes);
    let is_loading_seasonal = Memo::new(move |_| seasonal_query.get().is_none());
    let seasonal_slugs = Memo::new(move |_| match seasonal_query.get() {
        Some(Ok(recipes)) => recipes
            .iter()
            .filter(|recipe| recipe.in_season)
            .map(|recipe| recipe.slug.clone())
            .collect::<HashSet<_>>(),
        _ => HashSet::new(),
    });

    let recipes = Memo::new(move |_| {
        let all = all_recipes.get()?;
        if is_seasonal.get() {
            let in_season = seasonal_slugs.with(|slugs| {
                all.iter()
                    .filter(|recipe| slugs.contains(&recipe.slug))
                    .cloned()
                    .collect::<Vec<_>>()
            });
            Some(Arc::new(in_season))
        } else {
            Some(all)
        }
    });
    let recipe_count = Memo::new(move |_| {
        recipes.with(|recipes| recipes.as_ref().map_or(0, |recipes| recipes.len()))
    });
    let total_pages = Memo::new(move |_| recipe_count.get().div_ceil(PAGE_SIZE));
    let paginated_recipes = Memo::new(move |_| {
        let page = page.get();
        recipes.with(|recipes| {
            recipes
                .as_ref()
                .map(|recipes| {
                    let start = (page.saturating_sub(1) * PAGE_SIZE).min(recipes.len());
                    let end = (page * PAGE_SIZE).min(recipes.len()).max(start);
                    recipes[start..end].to_vec()
                })
                .unwrap_or_default()
        })
    });

    let subtitle = move || {
        if is_loading.get() || (is_seasonal.get() && is_loading_seasonal.get()) {
            return "Chargement…".to_owned();
        }
        let count = recipe_count.get();
        let suffix = if is_seasonal.get() {
            " de saison".to_owned()
        } else {
            match active_cookbook.get() {
                Some(cookbook) => format!(" dans {}", cookbook.name),
                None => " dans votre livre".to_owned(),
            }
        };
        format!(
            "{count} recette{}{suffix}",
            if count != 1 { "s" } else { "" }
        )
    };

    let show_empty = move || {
        !is_loading.get()
            && !is_error.get()
            && !(is_seasonal.get() && is_loading_seasonal.get())
            && recipes.with(|recipes| recipes.as_ref().is_some_and(|recipes| recipes.is_empty()))
    };

    let cookbook_buttons = move || {
        cookbooks
            .get()
            .unwrap_or_default()
            .into_iter()
            .map(|cookbook| {
                let Cookbook { slug, name, manual } = cookbook;
                let active_slug = slug.clone();
                view! {
                    <button
                        class=move || {
                            filter_class(
                                selected_cookbook.with(|selected| selected.as_deref() == Some(active_slug.as_str())),
                            )
                        }
                        on:click=move |_| select_filter(Filter::Cookbook(slug.clone()))
                    >
                        {(!manual).then_some("🏷 ")}
                        {name}
                    </button>
                }
            })
            .collect_view()
    };

    let category_options = move || {
        categories
            .get()
            .unwrap_or_default()
            .into_iter()
            .map(|category| {
                let RecipeCategory { id, name } = category;
                let option_id = id.clone();
                // Comme le `<select value>` contrôlé de React, qui coche l'option
                // correspondante (`option.selected`).
                view! {
                    <option
                        value=id
                        prop:selected=move || new_cookbook_category_id.with(|selected| *selected == option_id)
                    >
                        {name}
                    </option>
                }
            })
            .collect_view()
    };

    let recipe_card = move |recipe: RecipeSummary| {
        let RecipeSummary {
            slug,
            name,
            time,
            tag,
            image_url,
            color,
            is_favorite: _,
        } = recipe;
        let has_photo = image_url.as_deref().is_some_and(|url| !url.is_empty());
        let href = format!("/recette/{slug}");
        let favorite_slug = slug.clone();
        let is_favorite = Memo::new({
            let slug = slug.clone();
            move |_| favorite_slugs.with(|slugs| slugs.contains(&slug))
        });
        let remove_slug = slug.clone();
        let photo = if has_photo {
            view! { <img class="liste-cardPhoto" src=image_url alt="" /> }.into_any()
        } else {
            view! {
                <div class="liste-noPhoto">
                    <span>"Pas de photo"</span>
                </div>
            }
            .into_any()
        };
        let time = time.filter(|time| !time.is_empty()).map(|time| {
            view! {
                <span>{time}</span>
                <span>"·"</span>
            }
        });
        view! {
            <div class="liste-card">
                <a
                    href=href.clone()
                    class="liste-cardImage"
                    style:background=(!has_photo).then_some(color)
                >
                    {photo}
                    <button
                        aria-label=move || {
                            if is_favorite.get() { "Retirer des favoris" } else { "Ajouter aux favoris" }
                        }
                        class="liste-favoriteButton"
                        disabled=move || favorite.pending().get()
                        on:click=move |ev| {
                            ev.prevent_default();
                            ev.stop_propagation();
                            favorite.dispatch((favorite_slug.clone(), !is_favorite.get_untracked()));
                        }
                    >
                        <svg
                            width="15"
                            height="15"
                            viewBox="0 0 24 24"
                            fill=move || if is_favorite.get() { "#C1592F" } else { "none" }
                            stroke="#C1592F"
                            stroke-width="2"
                        >
                            <path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.6l-1-1a5.5 5.5 0 0 0-7.8 7.8l1 1L12 21l7.8-7.8 1-1a5.5 5.5 0 0 0 0-7.6z" />
                        </svg>
                    </button>
                    {move || {
                        seasonal_slugs
                            .with(|slugs| slugs.contains(&slug))
                            .then(|| view! { <span class="liste-seasonBadge">"🌱 De saison"</span> })
                    }}
                </a>
                <div class="liste-cardBody">
                    <a href=href class="liste-cardName">
                        {name}
                    </a>
                    <div class="liste-cardMeta">
                        {time}
                        <span>{tag.unwrap_or_else(|| "Sans catégorie".to_owned())}</span>
                    </div>
                    {(!has_photo)
                        .then(|| {
                            view! { <button class="liste-suggestButton">"+ Suggérer une photo"</button> }
                        })}
                    {move || {
                        can_remove_from_book
                            .get()
                            .then(|| {
                                let remove_slug = remove_slug.clone();
                                view! {
                                    <button
                                        class="liste-removeFromBookButton"
                                        disabled=move || remove_from_cookbook.pending().get()
                                        on:click=move |_| {
                                            remove_from_cookbook.dispatch(remove_slug.clone());
                                        }
                                    >
                                        "Retirer du livre"
                                    </button>
                                }
                            })
                    }}
                </div>
            </div>
        }
    };

    view! {
        <div class="liste-header">
            <div>
                <h1 class="liste-title">"Recettes"</h1>
                <p class="liste-subtitle">{subtitle}</p>
            </div>
            <div class="liste-search">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#8A7F6E" stroke-width="2">
                    <circle cx="11" cy="11" r="8" />
                    <path d="m21 21-4.3-4.3" />
                </svg>
                <input
                    class="liste-searchInput"
                    placeholder="Rechercher une recette…"
                    prop:value=move || search.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        if search.with_untracked(|search| *search != value) {
                            search.set(value);
                            page.set(1);
                        }
                    }
                />
            </div>
        </div>

        <div class="liste-filters">
            <button
                class=move || filter_class(is_seasonal.get())
                on:click=move |_| select_filter(Filter::Seasonal)
            >
                "🌱 De saison"
            </button>
            <button class=move || filter_class(is_all.get()) on:click=move |_| select_filter(Filter::All)>
                "Tout"
            </button>
            {cookbook_buttons}
            <div class="liste-newCookbookWrapper">
                <Show
                    when=move || creating_cookbook.get()
                    fallback=move || {
                        view! {
                            <button class="liste-filter" on:click=move |_| creating_cookbook.set(true)>
                                "+ Nouveau livre"
                            </button>
                        }
                    }
                >
                    <div node_ref=new_cookbook_panel_ref class="liste-newCookbookPanel">
                        <input
                            node_ref=new_cookbook_input_ref
                            class="liste-newCookbookInput"
                            placeholder="Nom du livre…"
                            prop:value=move || new_cookbook_name.get()
                            on:input=move |ev| new_cookbook_name.set(event_target_value(&ev))
                        />
                        <div class="liste-newCookbookModeRow">
                            <label class="liste-newCookbookModeOption">
                                <input
                                    type="radio"
                                    prop:checked=move || new_cookbook_mode.get() == NewCookbookMode::Manual
                                    on:change=move |_| new_cookbook_mode.set(NewCookbookMode::Manual)
                                />
                                "Manuel — j'ajoute les recettes moi-même"
                            </label>
                            <label class="liste-newCookbookModeOption">
                                <input
                                    type="radio"
                                    prop:checked=move || new_cookbook_mode.get() == NewCookbookMode::Category
                                    on:change=move |_| new_cookbook_mode.set(NewCookbookMode::Category)
                                />
                                "Automatique — toutes les recettes d'une catégorie"
                            </label>
                        </div>
                        {move || {
                            is_category_mode
                                .get()
                                .then(|| {
                                    view! {
                                        <select
                                            class="liste-newCookbookCategorySelect"
                                            prop:value=move || new_cookbook_category_id.get()
                                            on:change=move |ev| new_cookbook_category_id.set(event_target_value(&ev))
                                        >
                                            <option
                                                value=""
                                                disabled=true
                                                prop:selected=move || new_cookbook_category_id.with(String::is_empty)
                                            >
                                                "Choisir une catégorie…"
                                            </option>
                                            {category_options}
                                        </select>
                                    }
                                })
                        }}
                        <div class="liste-newCookbookActions">
                            <button class="liste-newCookbookCancel" on:click=move |_| reset_new_cookbook_form()>
                                "Annuler"
                            </button>
                            <button
                                class="liste-newCookbookConfirm"
                                disabled=move || {
                                    new_cookbook_name.with(|name| js::trim(name).is_empty())
                                        || (is_category_mode.get() && new_cookbook_category_id.with(String::is_empty))
                                        || create_cookbook.pending().get()
                                }
                                on:click=move |_| {
                                    create_cookbook.dispatch(());
                                }
                            >
                                {move || if create_cookbook.pending().get() { "Création…" } else { "Créer" }}
                            </button>
                        </div>
                    </div>
                </Show>
            </div>
        </div>

        <Show when=move || is_error.get()>
            <p class="liste-error">
                "Impossible de charger les recettes depuis Mealie. Vérifiez que Mealie est bien joignable."
            </p>
        </Show>

        <Show when=show_empty>
            <p class="liste-empty">
                {move || {
                    if is_seasonal.get() {
                        "Aucune recette de saison trouvée pour le moment."
                    } else {
                        "Aucune recette ne correspond à votre recherche."
                    }
                }}
            </p>
        </Show>

        <div class="liste-scrollArea">
            <div class="liste-grid">
                <For each=move || paginated_recipes.get() key=card_key children=recipe_card />
            </div>
            <Show when=move || { total_pages.get() > 1 }>
                <div class="liste-pagination">
                    <button
                        class="liste-paginationBtn"
                        disabled=move || page.get() == 1
                        on:click=move |_| page.update(|page| *page = page.saturating_sub(1))
                    >
                        "← Précédent"
                    </button>
                    <span class="liste-paginationInfo">
                        "Page " {move || page.get()} " / " {move || total_pages.get()}
                    </span>
                    <button
                        class="liste-paginationBtn"
                        disabled=move || page.get() == total_pages.get()
                        on:click=move |_| page.update(|page| *page += 1)
                    >
                        "Suivant →"
                    </button>
                </div>
            </Show>
        </div>
    }
}
