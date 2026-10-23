use leptos::prelude::*;

use crate::api::{self, COURSE_TYPES};

fn course_type_label(course_type: &str) -> &'static str {
    match course_type {
        "plat" => "Plat principal",
        "entrée" => "Entrée",
        "dessert" => "Dessert",
        "accompagnement" => "Accompagnement",
        _ => "",
    }
}

#[component]
pub fn Categories() -> impl IntoView {
    let categories = LocalResource::new(api::fetch_cookbook_categories);
    // Types de service : stockés dans le navigateur, lus sans attente.
    let course_types = RwSignal::new(api::fetch_course_types().ok());

    let mutation = Action::new_local(
        move |(name, course_type): &(String, Option<&'static str>)| {
            let result = api::set_course_type(name, *course_type);
            async move {
                if let Ok(data) = &result {
                    course_types.set(Some(data.clone()));
                }
                result
            }
        },
    );

    let category_rows = move || {
        categories.get().and_then(Result::ok).map(|categories| {
            categories
                .into_iter()
                .map(|category| {
                    let name = category.name.clone();
                    let current_type = Memo::new(move |_| {
                        course_types.with(|types| {
                            types
                                .as_ref()
                                .and_then(|types| types.get(&name).cloned())
                                .unwrap_or_default()
                        })
                    });
                    let buttons = COURSE_TYPES
                        .iter()
                        .map(|&course_type| {
                            let name = category.name.clone();
                            view! {
                                <button
                                    class=move || {
                                        if current_type.get() == course_type {
                                            "categories-typeBtnActive"
                                        } else {
                                            "categories-typeBtn"
                                        }
                                    }
                                    on:click=move |_| {
                                        let next = (current_type.get() != course_type).then_some(course_type);
                                        mutation.dispatch((name.clone(), next));
                                    }
                                    disabled=move || mutation.pending().get()
                                >
                                    {course_type_label(course_type)}
                                </button>
                            }
                        })
                        .collect_view();
                    view! {
                        <div class="categories-row">
                            <span class="categories-categoryName">{category.name}</span>
                            <div class="categories-typeSelector">{buttons}</div>
                        </div>
                    }
                })
                .collect_view()
        })
    };

    view! {
        <div class="categories-header">
            <div>
                <h1 class="categories-title">"Catégories"</h1>
                <p class="categories-subtitle">
                    "Définissez le type de service de chaque catégorie pour guider le planificateur IA."
                </p>
            </div>
        </div>

        <p class="categories-hint">
            "Les catégories marquées " <strong>"Dessert"</strong> " ou "
            <strong>"Accompagnement"</strong>
            " ne seront pas proposées par l'IA comme plat principal lors de la génération du planning."
        </p>

        <Show when=move || categories.get().is_none()>
            <p class="categories-loading">"Chargement…"</p>
        </Show>

        <div class="categories-list">{category_rows}</div>
    }
}
