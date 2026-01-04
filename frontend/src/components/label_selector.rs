use gloo_net::http::Request;
use leptos::*;
use wasm_bindgen::JsCast;

#[component]
pub fn LabelSelector(
    #[prop(into)] is_open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_select: Callback<String>,
) -> impl IntoView {
    let (labels, set_labels) = create_signal(Vec::<String>::new());
    let (filter, set_filter) = create_signal(String::new());
    let (selected_index, set_selected_index) = create_signal(0);

    // Fetch labels on mount
    create_effect(move |_| {
        spawn_local(async move {
            let url = "/labels";
            match Request::get(url).send().await {
                Ok(resp) => {
                    if let Ok(data) = resp.json::<Vec<String>>().await {
                        set_labels.set(data);
                    }
                }
                Err(e) => leptos::logging::error!("Failed to fetch labels: {:?}", e),
            }
        });
    });

    let filtered_labels = move || {
        let f = filter.get().to_lowercase();
        let all = labels.get();
        if f.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|l| l.to_lowercase().contains(&f))
                .collect()
        }
    };

    // Reset filter when opened
    create_effect(move |_| {
        if is_open.get() {
            set_filter.set("".to_string());
            set_selected_index.set(0);

            // Focus input
            set_timeout(
                move || {
                    if let Some(doc) = document().dyn_into::<web_sys::Document>().ok() {
                        if let Some(el) = doc.get_element_by_id("label-selector-input") {
                            let _ = el.dyn_into::<web_sys::HtmlElement>().map(|h| h.focus());
                        }
                    }
                },
                std::time::Duration::from_millis(50),
            );
        }
    });

    let select_current = move || {
        let list = filtered_labels();
        if !list.is_empty() {
            let idx = selected_index.get();
            if idx < list.len() {
                on_select.call(list[idx].clone());

                // Blur input explicitly before closing to return focus to body/window
                if let Some(doc) = document().dyn_into::<web_sys::Document>().ok() {
                    if let Some(el) = doc.get_element_by_id("label-selector-input") {
                        let _ = el.dyn_into::<web_sys::HtmlElement>().map(|h| h.blur());
                    }
                }

                on_close.call(());
            }
        }
    };

    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let list = filtered_labels();
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                if !list.is_empty() {
                    set_selected_index.update(|i| *i = (*i + 1).min(list.len() - 1));
                }
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = i.saturating_sub(1));
            }
            "Enter" => {
                ev.prevent_default();
                select_current();
            }
            "Escape" => {
                on_close.call(());
            }
            _ => {}
        }
    };

    view! {
        <div class=move || {
            let base = "fixed inset-0 z-50 flex items-start pt-32 justify-center bg-black/20 backdrop-blur-sm transition-opacity duration-200";
            if is_open.get() {
                format!("{} opacity-100 pointer-events-auto", base)
            } else {
                format!("{} opacity-0 pointer-events-none", base)
            }
        }
        on:click=move |_| on_close.call(())
        >
            <div
                class="bg-white rounded-xl shadow-2xl w-full max-w-lg overflow-hidden flex flex-col"
                on:click=move |ev| ev.stop_propagation()
            >
                <div class="p-4 border-b">
                     <h2 class="text-lg font-bold mb-2 text-gray-700">"Go to Label"</h2>
                     <input
                        id="label-selector-input"
                        type="text"
                        class="w-full bg-gray-100 border border-gray-300 rounded-lg px-4 py-2 focus:bg-white focus:border-blue-500 outline-none text-lg"
                        placeholder="Type label name..."
                        prop:value=filter
                        on:input=move |ev| {
                            set_filter.set(event_target_value(&ev));
                            set_selected_index.set(0);
                        }
                        on:keydown=handle_keydown
                     />
                </div>
                <div class="max-h-96 overflow-y-auto p-2">
                     <For
                        each=filtered_labels
                        key=|label| label.clone()
                        children=move |label| {
                            let label_for_class = label.clone();
                            let label_for_click = label.clone();
                            let label_for_enter = label.clone();
                            let label_for_view = label.clone();
                            let label_for_check = label.clone();

                            view! {
                                <div
                                    class=move || {
                                        let base = "px-4 py-2 rounded-lg cursor-pointer flex items-center justify-between";
                                        let list = filtered_labels();
                                        let idx = list.iter().position(|l| l == &label_for_class).unwrap_or(0);
                                        if idx == selected_index.get() {
                                            format!("{} bg-blue-100 text-blue-800 font-semibold", base)
                                        } else {
                                            format!("{} hover:bg-gray-50 text-gray-700", base)
                                        }
                                    }
                                    on:click=move |_| {
                                        on_select.call(label_for_click.clone());
                                        on_close.call(());
                                    }
                                    on:mouseenter=move |_| {
                                         let list = filtered_labels();
                                         if let Some(pos) = list.iter().position(|l| l == &label_for_enter) {
                                             set_selected_index.set(pos);
                                         }
                                    }
                                >
                                    <span>{label_for_view}</span>
                                    {move || {
                                        if selected_index.get() == filtered_labels().iter().position(|l| l == &label_for_check).unwrap_or(0) {
                                            view! { <span class="text-xs text-blue-500">"↵"</span> }.into_view()
                                        } else {
                                            view! {}.into_view()
                                        }
                                    }}
                                </div>
                            }
                        }
                     />
                     {move || {
                         if filtered_labels().is_empty() {
                             view! { <div class="text-center text-gray-400 py-4">"No matching labels"</div> }.into_view()
                         } else {
                             view! {}.into_view()
                         }
                     }}
                </div>
            </div>
        </div>
    }
}
