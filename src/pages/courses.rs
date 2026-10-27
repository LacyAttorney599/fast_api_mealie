//! Liste de courses (ex-`frontend/src/pages/Courses.tsx`).

use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_query_map};
use leptos_router::params::ParamsMap;

use crate::api::{self, ShoppingCategory, ShoppingItem, ShoppingList};
use crate::app::{bump, use_refresh};
use crate::util::{dates, js};

/// `searchParams.get(key)` : première valeur du paramètre, comme `URLSearchParams`.
fn first_param(params: &ParamsMap, key: &str) -> Option<String> {
    params
        .get_all(key)
        .and_then(|values| values.into_iter().next())
}

#[component]
pub fn Courses() -> impl IntoView {
    let query = use_query_map();
    let refresh = use_refresh();
    let adding_item = RwSignal::new(false);
    let new_item_text = RwSignal::new(String::new());

    let this_monday = move || dates::to_iso_date(dates::get_monday(dates::today()));
    let start = Memo::new(move |_| {
        query
            .with(|q| first_param(q, "start"))
            .unwrap_or_else(this_monday)
    });
    let end = Memo::new(move |_| {
        query
            .with(|q| first_param(q, "end"))
            .unwrap_or_else(this_monday)
    });
    // `new Date(start)` : une date invalide donnait « NaN – NaN undefined NaN »
    // en React ; on retombe ici sur le lundi de la semaine en cours.
    let week_label = Memo::new(move |_| {
        let monday = start
            .with(|start| dates::parse_iso_date(start))
            .unwrap_or_else(|| dates::get_monday(dates::today()));
        dates::format_week_range(monday)
    });

    // useQuery(["shoppinglist"]) : `fetching` reproduit `isFetching`, et
    // `list` garde les dernières données reçues (React Query les conserve
    // quand un rechargement échoue).
    let fetching = RwSignal::new(false);
    let resource = LocalResource::new(move || {
        refresh.shopping_list.track();
        async move {
            fetching.set(true);
            let result = api::fetch_shopping_list().await;
            fetching.set(false);
            result
        }
    });
    let list = Memo::new(
        move |previous: Option<&Option<ShoppingList>>| match resource.get() {
            Some(Ok(list)) => Some(list),
            _ => previous.cloned().flatten(),
        },
    );
    // isLoading = pas encore de données et chargement en cours.
    let is_loading = Memo::new(move |_| {
        list.with(Option::is_none) && (fetching.get() || resource.with(Option::is_none))
    });
    let category_count =
        Memo::new(move |_| list.with(|list| list.as_ref().map_or(0, |list| list.categories.len())));
    let total_items =
        Memo::new(move |_| list.with(|list| list.as_ref().map_or(0, |list| list.total_items)));
    let recipe_count =
        Memo::new(move |_| list.with(|list| list.as_ref().map_or(0, |list| list.recipe_count)));
    // `!isLoading && list && list.categories.length === 0` / `> 0`.
    let show_empty = Memo::new(move |_| {
        !is_loading.get() && list.with(Option::is_some) && category_count.get() == 0
    });
    let show_body = Memo::new(move |_| {
        !is_loading.get() && list.with(Option::is_some) && category_count.get() > 0
    });
    let categories = Memo::new(move |_| {
        list.with(|list| {
            list.as_ref()
                .map(|list| list.categories.clone())
                .unwrap_or_default()
        })
    });

    let generate = Action::new_local(move |(start, end): &(String, String)| {
        let (start, end) = (start.clone(), end.clone());
        async move {
            let result = api::generate_shopping_list(&start, &end).await;
            if result.is_ok() {
                bump(refresh.shopping_list);
            }
            result
        }
    });
    let toggle = Action::new_local(move |(id, checked): &(String, bool)| {
        let (id, checked) = (id.clone(), *checked);
        async move {
            let result = api::toggle_shopping_item(&id, checked).await;
            if result.is_ok() {
                bump(refresh.shopping_list);
            }
            result
        }
    });
    let clear_checked = Action::new_local(move |_: &()| async move {
        let result = api::clear_checked_shopping_items().await;
        if result.is_ok() {
            bump(refresh.shopping_list);
        }
        result
    });
    let clear_all = Action::new_local(move |_: &()| async move {
        let result = api::clear_all_shopping_items().await;
        if result.is_ok() {
            bump(refresh.shopping_list);
        }
        result
    });
    let add = Action::new_local(move |text: &String| {
        let text = text.clone();
        async move {
            let result = api::add_shopping_item(&text).await;
            if result.is_ok() {
                bump(refresh.shopping_list);
                new_item_text.set(String::new());
                adding_item.set(false);
            }
            result
        }
    });

    // Pendant la génération, les boutons sont désactivés : un second clic
    // ajouterait les mêmes recettes une deuxième fois. Un échec est signalé.
    let generate_pending = generate.pending();
    let generate_failed = move || {
        !generate_pending.get() && generate.value().with(|value| matches!(value, Some(Err(_))))
    };
    let generate_from_planning = move |_| {
        if !generate_pending.get_untracked() {
            generate.dispatch((start.get_untracked(), end.get_untracked()));
        }
    };

    let handle_clear_all = move |_| {
        if window()
            .confirm_with_message("Vider toute la liste de courses ?")
            .unwrap_or(false)
        {
            clear_all.dispatch(());
        }
    };

    let submit_new_item = move || {
        let text = new_item_text.try_get_untracked().unwrap_or_default();
        let text = js::trim(&text);
        if text.is_empty() {
            adding_item.set(false);
        } else {
            add.dispatch(text.to_owned());
        }
    };

    // Quitter la page retire le champ du DOM, ce qui peut déclencher `blur` ;
    // React n'appelait pas `onBlur` dans ce cas : rien ne doit être ajouté.
    let location = use_location();
    let page_path = StoredValue::new(location.pathname.get_untracked());
    let still_on_page = move || {
        location
            .pathname
            .try_get_untracked()
            .is_some_and(|path| page_path.try_with_value(|page| *page == path) == Some(true))
    };

    // autoFocus : le champ est recréé à chaque ouverture.
    let add_input = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if let Some(input) = add_input.get() {
            let _ = input.focus();
        }
    });

    let item_row = move |items: Memo<Vec<ShoppingItem>>, item: ShoppingItem| {
        let id = item.id.clone();
        let current = Memo::new(move |_| {
            items.with(|items| items.iter().find(|item| item.id == id).cloned())
        });
        let checked =
            Memo::new(move |_| current.with(|item| item.as_ref().is_some_and(|item| item.checked)));
        let quantity = Memo::new(move |_| {
            current.with(|item| {
                item.as_ref()
                    .map(|item| item.quantity.clone())
                    .unwrap_or_default()
            })
        });
        let food = Memo::new(move |_| {
            current.with(|item| {
                item.as_ref()
                    .map(|item| item.food.clone())
                    .unwrap_or_default()
            })
        });
        let id = item.id;
        view! {
            <div class="courses-itemRow">
                <input
                    type="checkbox"
                    prop:checked=move || checked.get()
                    on:change=move |ev| {
                        let input = event_target::<web_sys::HtmlInputElement>(&ev);
                        let wanted = input.checked();
                        // Case contrôlée : elle garde l'état de la liste jusqu'au rechargement.
                        input.set_checked(checked.get_untracked());
                        toggle.dispatch((id.clone(), wanted));
                    }
                    class="courses-checkbox"
                />
                {move || {
                    let quantity = quantity.get();
                    (!quantity.is_empty()).then(|| view! { <span class="courses-itemQty">{quantity}</span> })
                }}
                <span class=move || {
                    if checked.get() { "courses-itemNameChecked" } else { "courses-itemName" }
                }>{move || food.get()}</span>
            </div>
        }
    };

    let category_block = move |category: ShoppingCategory| {
        let name = category.name.clone();
        let items = Memo::new(move |_| {
            categories.with(|categories| {
                categories
                    .iter()
                    .find(|category| category.name == name)
                    .map(|category| category.items.clone())
                    .unwrap_or_default()
            })
        });
        view! {
            <div class="courses-category">
                <span class="courses-categoryName">{category.name}</span>
                <For
                    each=move || items.get()
                    key=|item| item.id.clone()
                    children=move |item| item_row(items, item)
                />
            </div>
        }
    };

    view! {
        <div class="courses-header">
            <div>
                <h1 class="courses-title">"Liste de courses"</h1>
                <p class="courses-subtitle">"Générée depuis le planning du " {move || week_label.get()} "."</p>
            </div>
            <div class="courses-headerActions">
                <button class="courses-clearButton" on:click=move |_| { clear_checked.dispatch(()); }>
                    "Effacer les articles cochés"
                </button>
                <button class="courses-clearAllButton" on:click=handle_clear_all>
                    "Vider la liste"
                </button>
                <Show
                    when=move || adding_item.get()
                    fallback=move || {
                        view! {
                            <button class="courses-addButton" on:click=move |_| adding_item.set(true)>
                                "+ Ajouter un article"
                            </button>
                        }
                    }
                >
                    <input
                        node_ref=add_input
                        class="courses-addInput"
                        prop:value=move || new_item_text.get()
                        placeholder="Nom de l'article…"
                        on:input=move |ev| new_item_text.set(event_target_value(&ev))
                        on:blur=move |_| {
                            if still_on_page() {
                                submit_new_item();
                            }
                        }
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                submit_new_item();
                            }
                        }
                    />
                </Show>
            </div>
        </div>

        <Show when=generate_failed>
            <p class="courses-errorText">
                "Échec de la génération de la liste de courses. Vérifiez que Mealie est joignable."
            </p>
        </Show>

        <Show when=move || is_loading.get()>
            <p class="courses-status">"Chargement…"</p>
        </Show>

        <Show when=move || show_empty.get()>
            <div class="courses-emptyState">
                <p>"La liste est vide."</p>
                <button
                    class="courses-generateInline"
                    disabled=move || generate_pending.get()
                    on:click=generate_from_planning
                >
                    {move || {
                        if generate_pending.get() {
                            "Génération en cours…".to_owned()
                        } else {
                            format!("Générer depuis le planning du {}", week_label.get())
                        }
                    }}
                </button>
            </div>
        </Show>

        <Show when=move || show_body.get()>
            <div class="courses-body">
                <div class="courses-categories">
                    <For
                        each=move || categories.get()
                        key=|category| category.name.clone()
                        children=category_block
                    />
                </div>

                <div class="courses-sidebar">
                    <div class="courses-summaryCard">
                        <span class="courses-summaryLabel">"Résumé"</span>
                        <span class="courses-summaryValue">{move || total_items.get()} " articles"</span>
                        <span class="courses-summaryDetail">
                            {move || category_count.get()} " catégories · " {move || recipe_count.get()}
                            " recette" {move || if recipe_count.get() != 1 { "s" } else { "" }}
                            " du planning"
                        </span>
                    </div>
                    <div class="courses-infoCard">
                        <span class="courses-infoTitle">"Fusion automatique"</span>
                        <span class="courses-infoText">
                            "Les quantités identiques sont additionnées automatiquement entre les recettes du planning."
                        </span>
                    </div>
                    <button
                        class="courses-generateButton"
                        disabled=move || generate_pending.get()
                        on:click=generate_from_planning
                    >
                        {move || {
                            if generate_pending.get() { "Génération en cours…" } else { "Régénérer depuis le planning" }
                        }}
                    </button>
                    <button class="courses-shareButton">"Partager la liste"</button>
                </div>
            </div>
        </Show>
    }
}
