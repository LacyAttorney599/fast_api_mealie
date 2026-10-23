use leptos::prelude::*;
use leptos_router::hooks::use_location;

struct NavItem {
    to: &'static str,
    label: &'static str,
    icon: fn() -> AnyView,
    is_active: fn(&str) -> bool,
}

const NAV_ITEMS: [NavItem; 5] = [
    NavItem {
        to: "/",
        label: "Recettes",
        icon: || {
            view! {
                <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
            }
            .into_any()
        },
        is_active: |pathname| pathname == "/" || pathname.starts_with("/recette"),
    },
    NavItem {
        to: "/planificateur",
        label: "Planificateur",
        icon: || {
            view! {
                <rect x="3" y="4" width="18" height="18" rx="2" />
                <path d="M16 2v4M8 2v4M3 10h18" />
            }
            .into_any()
        },
        is_active: |pathname| pathname.starts_with("/planificateur"),
    },
    NavItem {
        to: "/courses",
        label: "Liste de courses",
        icon: || {
            view! {
                <circle cx="9" cy="21" r="1" />
                <circle cx="20" cy="21" r="1" />
                <path d="M1 1h4l2.68 13.39a2 2 0 0 0 2 1.61h9.72a2 2 0 0 0 2-1.61L23 6H6" />
            }
            .into_any()
        },
        is_active: |pathname| pathname.starts_with("/courses"),
    },
    NavItem {
        to: "/importer",
        label: "Importer",
        icon: || {
            view! {
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <path d="M17 8l-5-5-5 5M12 3v12" />
            }
            .into_any()
        },
        is_active: |pathname| pathname.starts_with("/importer"),
    },
    NavItem {
        to: "/categories",
        label: "Catégories",
        icon: || view! { <path d="M4 6h16M4 12h8M4 18h12" /> }.into_any(),
        is_active: |pathname| pathname.starts_with("/categories"),
    },
];

/// `aria-current="page"` de `NavLink` : lien vers la page courante ou l'une
/// de ses sous-pages (la racine ne correspond qu'à elle-même).
fn aria_current(to: &str, pathname: &str) -> Option<&'static str> {
    let matches = pathname == to
        || (to != "/" && pathname.strip_prefix(to).is_some_and(|rest| rest.starts_with('/')));
    matches.then_some("page")
}

#[component]
pub fn Sidebar() -> impl IntoView {
    let pathname = use_location().pathname;

    view! {
        <div class="sidebar-sidebar">
            <div class="sidebar-brand">
                <div class="sidebar-logo">
                    <svg
                        width="19"
                        height="19"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="#FAF7F2"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M18 2v6a4 4 0 0 1-4 4H6" />
                        <path d="M6 2v20" />
                        <path d="M2 2v6a4 4 0 0 0 4 4" />
                    </svg>
                </div>
                <span class="sidebar-brandName">"Ma Cuisine"</span>
            </div>

            <nav class="sidebar-nav">
                {NAV_ITEMS
                    .iter()
                    .map(|item| {
                        let to = item.to;
                        let is_active = item.is_active;
                        view! {
                            <a
                                href=to
                                class="sidebar-navLink"
                                class:sidebar-navLinkActive=move || is_active(&pathname.get())
                                aria-current=move || pathname.with(|pathname| aria_current(to, pathname))
                            >
                                <svg
                                    width="19"
                                    height="19"
                                    viewBox="0 0 24 24"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="2"
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                >
                                    {(item.icon)()}
                                </svg>
                                {item.label}
                            </a>
                        }
                    })
                    .collect_view()}
            </nav>

            <div class="sidebar-footer">
                <a
                    href="/ajouter"
                    class="sidebar-newRecipeButton"
                    aria-current=move || pathname.with(|pathname| aria_current("/ajouter", pathname))
                >
                    <svg
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="#FFFFFF"
                        stroke-width="2.5"
                        stroke-linecap="round"
                    >
                        <path d="M12 5v14M5 12h14" />
                    </svg>
                    "Nouvelle recette"
                </a>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aria_current_follows_navlink_matching() {
        assert_eq!(aria_current("/", "/"), Some("page"));
        assert_eq!(aria_current("/", "/recette/soupe"), None);
        assert_eq!(aria_current("/courses", "/courses"), Some("page"));
        assert_eq!(aria_current("/courses", "/courses/x"), Some("page"));
        assert_eq!(aria_current("/courses", "/coursesx"), None);
    }
}
