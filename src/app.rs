//! Routes de l'application et état partagé entre les écrans.

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::path;

use crate::api::RecipeDraft;
use crate::components::Layout;
use crate::pages::{Ajouter, Categories, Courses, Detail, Importer, Liste, Planificateur};

/// Remplace les `invalidateQueries` de React Query : les ressources lisent
/// ces compteurs, et une modification les incrémente (avec [`bump`]) pour
/// forcer le rechargement des données concernées.
#[derive(Clone, Copy)]
pub struct Refresh {
    /// Liste des recettes (recherche, favoris, livres).
    pub recipes: RwSignal<u32>,
    pub cookbooks: RwSignal<u32>,
    pub mealplan: RwSignal<u32>,
    pub shopping_list: RwSignal<u32>,
}

pub fn use_refresh() -> Refresh {
    expect_context()
}

pub fn bump(counter: RwSignal<u32>) {
    counter.update(|n| *n += 1);
}

/// Brouillon produit par l'import photo, repris par le formulaire d'ajout
/// (l'ancien `navigate("/ajouter", { state: draft })`).
#[derive(Clone, Copy)]
pub struct PendingDraft(pub RwSignal<Option<RecipeDraft>>);

pub fn use_pending_draft() -> PendingDraft {
    expect_context()
}

#[component]
pub fn App() -> impl IntoView {
    provide_context(Refresh {
        recipes: RwSignal::new(0),
        cookbooks: RwSignal::new(0),
        mealplan: RwSignal::new(0),
        shopping_list: RwSignal::new(0),
    });
    provide_context(PendingDraft(RwSignal::new(None)));

    view! {
        <Router>
            <Routes fallback=|| ()>
                <ParentRoute path=path!("") view=Layout>
                    <Route path=path!("") view=Liste />
                    <Route path=path!("recette/:slug") view=Detail />
                    <Route path=path!("recette/:slug/modifier") view=Ajouter />
                    <Route path=path!("ajouter") view=Ajouter />
                    <Route path=path!("importer") view=Importer />
                    <Route path=path!("planificateur") view=Planificateur />
                    <Route path=path!("courses") view=Courses />
                    <Route path=path!("categories") view=Categories />
                </ParentRoute>
            </Routes>
        </Router>
    }
}
