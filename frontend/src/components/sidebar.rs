use gloo_net::http::Request;
use leptos::*;

#[component]
pub fn Sidebar(
    selected_label: ReadSignal<String>,
    #[prop(into)] on_select_label: Callback<String>,
) -> impl IntoView {
    let (labels, set_labels) = create_signal(Vec::<String>::new());

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

    let standard = vec![
        "Inbox",
        "Sent",
        "Trash",
        "Spam",
        "Drafts",
        "Important",
        "Starred",
    ];

    view! {
        <div class="w-64 bg-white border-r h-full flex flex-col overflow-y-auto">
            <div class="p-4 text-xl font-bold text-red-600">"Gmail Archive"</div>
            <nav class="flex-1">
                <button
                    on:click=move |_| on_select_label.call("ALL".to_string())
                    class=move || {
                        let base = "w-full text-left px-6 py-2 hover:bg-gray-100 rounded-r-full mr-2";
                        if selected_label.get() == "ALL" {
                            format!("{} bg-red-50 text-red-600 font-semibold", base)
                        } else {
                            format!("{} text-gray-700", base)
                        }
                    }
                >
                    "ALL"
                </button>

                {move || {
                    let current_labels = labels.get();
                    // Need to own the strings for local filtering

                    let mut sys_labels = Vec::new();
                    let mut others = Vec::new();

                    for l in &current_labels {
                        if standard.contains(&l.as_str()) {
                            sys_labels.push(l.clone());
                        } else {
                            others.push(l.clone());
                        }
                    }
                    // Sort standard labels to match standard order?
                    // current_labels might be arbitrary.
                    // Let's filter standard list instead to keep order
                    let sorted_sys: Vec<String> = standard.iter()
                        .filter(|s| current_labels.contains(&s.to_string()))
                        .map(|s| s.to_string())
                        .collect();

                    // Remainder are others
                    // others is already populated above correctly (anything not in standard)

                    view! {
                         <For
                            each=move || sorted_sys.clone()
                            key=|label| label.to_string()
                            children=move |label| {
                                let label_str = label.clone();
                                view! {
                                    <SidebarButton
                                        label=label_str
                                        selected_label=selected_label
                                        on_click=on_select_label
                                    />
                                }
                            }
                        />

                        {if !others.is_empty() {
                            view! { <div class="mt-4 px-6 text-xs font-semibold text-gray-500 uppercase">"Labels"</div> }.into_view()
                        } else {
                            view! {}.into_view()
                        }}

                        <For
                            each=move || others.clone()
                            key=|label| label.to_string()
                            children=move |label| {
                                let label_str = label.clone();
                                view! {
                                    <SidebarButton
                                        label=label_str
                                        selected_label=selected_label
                                        on_click=on_select_label
                                    />
                                }
                            }
                        />

                    }
                }}
            </nav>
        </div>
    }
}

#[component]
fn SidebarButton(
    label: String,
    selected_label: ReadSignal<String>,
    #[prop(into)] on_click: Callback<String>,
) -> impl IntoView {
    let lbl_for_click = label.clone();
    let lbl_for_class = label.clone();
    let lbl_for_title = label.clone();
    let lbl_text = label.clone();

    view! {
        <button
            on:click=move |_| on_click.call(lbl_for_click.clone())
            class=move || {
                let base = "w-full text-left px-6 py-2 hover:bg-gray-100 rounded-r-full mr-2 truncate text-sm";
                if selected_label.get() == lbl_for_class {
                    format!("{} bg-red-50 text-red-600 font-semibold", base)
                } else {
                    format!("{} text-gray-700", base)
                }
            }
            title=lbl_for_title
        >
            {lbl_text}
        </button>
    }
}
