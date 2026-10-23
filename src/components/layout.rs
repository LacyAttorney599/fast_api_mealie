use leptos::prelude::*;
use leptos_router::components::Outlet;

use crate::components::Sidebar;

#[component]
pub fn Layout() -> impl IntoView {
    view! {
        <div class="layout-layout">
            <Sidebar />
            <main class="layout-content">
                <Outlet />
            </main>
        </div>
    }
}
