use crate::components::shortcut_handler::ShortcutAction;
use crate::{Email, SearchResponse};
use gloo_net::http::Request;
use leptos::html::Div;
use leptos::*;
use wasm_bindgen::prelude::*;
use web_sys::{IntersectionObserver, IntersectionObserverEntry, IntersectionObserverInit};

#[component]
pub fn EmailList(
    label: ReadSignal<String>,
    query: ReadSignal<String>,
    start_date: ReadSignal<String>,
    end_date: ReadSignal<String>,
    selected_email_id: ReadSignal<Option<String>>,
    #[prop(into)] on_select_email: Callback<String>,
    #[prop(into)] shortcut_signal: Signal<Option<ShortcutAction>>,
) -> impl IntoView {
    let (page, set_page) = create_signal(1);
    let (emails, set_emails) = create_signal(Vec::<Email>::new());
    let (loading, set_loading) = create_signal(false);
    let (has_more, set_has_more) = create_signal(true);
    let (is_sentinel_visible, set_is_sentinel_visible) = create_signal(false);

    // Reset when filters change
    create_effect(move |_| {
        let _ = label.get();
        let _ = query.get();
        let _ = start_date.get();
        let _ = end_date.get();

        set_page.set(1);
        set_emails.set(Vec::new());
        set_has_more.set(true);
        // Note: is_sentinel_visible is updated by observer independently
    });

    // Reactive Load More Logic
    // Triggers when: sentinel is visible AND not loading AND has more pages
    create_effect(move |_| {
        if is_sentinel_visible.get() && !loading.get() && has_more.get() {
            set_page.update(|p| *p += 1);
        }
    });

    // Shortcut Handling
    create_effect(move |_| {
        if let Some(action) = shortcut_signal.get() {
            let current_list = emails.get_untracked();
            if current_list.is_empty() {
                return;
            }

            let current_id = selected_email_id.get_untracked();

            match action {
                ShortcutAction::NextThread => {
                    // Find current index
                    let next_idx = match current_id {
                        Some(id) => current_list
                            .iter()
                            .position(|e| e.id == id)
                            .map(|i| i + 1)
                            .unwrap_or(0),
                        None => 0,
                    };

                    if next_idx < current_list.len() {
                        on_select_email.call(current_list[next_idx].id.clone());
                    }
                }
                ShortcutAction::PrevThread => {
                    let prev_idx = match current_id {
                        Some(id) => current_list
                            .iter()
                            .position(|e| e.id == id)
                            .map(|i| if i > 0 { i - 1 } else { 0 })
                            .unwrap_or(0),
                        None => 0,
                    };
                    on_select_email.call(current_list[prev_idx].id.clone());
                }
                ShortcutAction::GoToNextPage => {
                    // Jump 20 down
                    let current_idx = match current_id {
                        Some(id) => current_list.iter().position(|e| e.id == id).unwrap_or(0),
                        None => 0,
                    };
                    let next_idx = current_idx + 20;
                    if next_idx < current_list.len() {
                        on_select_email.call(current_list[next_idx].id.clone());
                    } else if !current_list.is_empty() {
                        on_select_email.call(current_list[current_list.len() - 1].id.clone());
                    }
                }
                ShortcutAction::GoToPrevPage => {
                    // Jump 20 up
                    let current_idx = match current_id {
                        Some(id) => current_list.iter().position(|e| e.id == id).unwrap_or(0),
                        None => 0,
                    };
                    let prev_idx = if current_idx > 20 {
                        current_idx - 20
                    } else {
                        0
                    };
                    on_select_email.call(current_list[prev_idx].id.clone());
                }
                _ => {}
            }

            if let Some(id) = selected_email_id.get_untracked() {
                set_timeout(
                    move || {
                        if let Some(doc) = document().dyn_into::<web_sys::Document>().ok() {
                            if let Some(el) = doc.get_element_by_id(&format!("email-row-{}", id)) {
                                let options = web_sys::ScrollIntoViewOptions::new();
                                options.set_block(web_sys::ScrollLogicalPosition::Nearest);
                                options.set_behavior(web_sys::ScrollBehavior::Smooth);
                                let _ = el.scroll_into_view_with_scroll_into_view_options(&options);
                            }
                        }
                    },
                    std::time::Duration::from_millis(10),
                );
            }
        }
    });

    // Fetch data
    create_effect(move |_| {
        let p = page.get();
        // Only fetch if we have filters or initial load.
        // Dependency tracking handles the rest.

        let l = label.get();
        let q = query.get();
        let s = start_date.get();
        let e = end_date.get();

        spawn_local(async move {
            set_loading.set(true);
            let mut url = format!("http://localhost:8001/search?page={}&size=50", p); // Increased page size
            if !l.is_empty() {
                url.push_str(&format!("&label={}", js_sys::encode_uri_component(&l)));
            }
            if !q.is_empty() {
                url.push_str(&format!("&q={}", js_sys::encode_uri_component(&q)));
            }
            if !s.is_empty() {
                url.push_str(&format!("&start_date={}", js_sys::encode_uri_component(&s)));
            }
            if !e.is_empty() {
                url.push_str(&format!("&end_date={}", js_sys::encode_uri_component(&e)));
            }

            match Request::get(&url).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<SearchResponse>().await {
                        let items = data.items;
                        set_has_more.set(items.len() == 50); // Matches page size

                        set_emails.update(|current| {
                            if p == 1 {
                                *current = items;
                            } else {
                                current.extend(items);
                            }
                        });
                    }
                }
                Err(err) => leptos::logging::error!("Fetch error: {:?}", err),
            }
            set_loading.set(false);
        });
    });

    let observer_ref = create_node_ref::<Div>();

    create_effect(move |_| {
        if let Some(el) = observer_ref.get() {
            let cb =
                Closure::<dyn FnMut(Vec<IntersectionObserverEntry>, IntersectionObserver)>::new(
                    move |entries: Vec<IntersectionObserverEntry>,
                          _observer: IntersectionObserver| {
                        if let Some(entry) = entries.first() {
                            set_is_sentinel_visible.set(entry.is_intersecting());
                        }
                    },
                );

            let options = IntersectionObserverInit::new();
            // root: null (viewport), threshold: 0.1 (trigger quickly)
            options.set_threshold(&JsValue::from(0.1));

            if let Ok(obs) =
                IntersectionObserver::new_with_options(cb.as_ref().unchecked_ref(), &options)
            {
                let _ = obs.observe(&el);
                cb.forget();
                let obs_clone = obs.clone();
                on_cleanup(move || {
                    obs_clone.disconnect();
                });
            }
        }
    });

    view! {
        <div class="flex-1 bg-white flex flex-col overflow-y-auto">
             {move || {
                let current_emails = emails.get();
                if current_emails.is_empty() && !loading.get() {
                    view! { <div class="p-8 text-center text-gray-500">"No emails found"</div> }.into_view()
                } else {
                    view! {}.into_view()
                }
             }}

             <For
                each=move || emails.get()
                key=|email| email.id.clone()
                children=move |email| {
                    let id = email.id.clone();
                    let id_attr = id.clone();
                    let id_clone = id.clone();
                    let is_selected_sig = selected_email_id;
                    let is_selected = move || is_selected_sig.get() == Some(id.clone());

                    // Simple Date Formatting (YYYY-MM-DD from ISO string)
                    let date_display = email.date.chars().take(10).collect::<String>();

                    view! {
                        <div
                            id=move || format!("email-row-{}", id_attr)
                            on:click=move |_| on_select_email.call(id_clone.clone())
                            class=move || {
                                if is_selected() {
                                    "border-b px-4 py-3 cursor-pointer hover:shadow-md transition-shadow flex items-center gap-4 bg-blue-50 border-l-4 border-l-blue-500"
                                } else {
                                    "border-b px-4 py-3 cursor-pointer hover:shadow-md transition-shadow flex items-center gap-4 hover:bg-gray-50"
                                }
                            }
                        >
                             <div class="w-48 font-semibold truncate text-gray-900">{email.sender}</div>
                             <div class="flex-1 min-w-0 flex items-center gap-2">
                                 <span class="font-medium text-gray-800 truncate">{email.subject}</span>
                                 {if email.has_attachment {
                                     view! { <span title="Has attachment">"📎"</span> }.into_view()
                                 } else {
                                     view! {}.into_view()
                                 }}
                                 <span class="text-gray-500 mx-1">"-"</span>
                                 <span class="text-gray-500 truncate">{email.snippet}</span>
                             </div>
                             // Revised Date Column
                             <div class="text-xs text-gray-500 font-medium whitespace-nowrap min-w-[100px] text-right">
                                 {date_display}
                             </div>
                        </div>
                    }
                }
             />

             // Sentinel / Loading Indicator
             <div _ref=observer_ref class="h-8 w-full flex justify-center items-center text-gray-400 text-sm p-2">
                 {move || if loading.get() { "Loading..." } else { "" }}
             </div>
        </div>
    }
}
