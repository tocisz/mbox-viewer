use leptos::*;
use gloo_net::http::Request;
use crate::EmailDetailData;

#[component]
pub fn EmailDetail(
    id: String,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] on_select_label: Callback<String>,
) -> impl IntoView {
    let (email, set_email) = create_signal::<Option<EmailDetailData>>(None);
    let (loading, set_loading) = create_signal(false);
    
    let id_clone = id.clone();
    create_effect(move |_| {
        let current_id = id_clone.clone();
        spawn_local(async move {
             set_loading.set(true);
             let url = format!("http://localhost:8001/email/{}", current_id);
             match Request::get(&url).send().await {
                 Ok(resp) => {
                     if let Ok(data) = resp.json::<EmailDetailData>().await {
                         set_email.set(Some(data));
                     }
                 },
                 Err(e) => leptos::logging::error!("Failed to fetch email: {:?}", e),
             }
             set_loading.set(false);
        });
    });

    view! {
        <div class="h-full flex flex-col bg-white p-6 overflow-y-auto relative">
             <button
                on:click=move |_| on_close.call(())
                class="absolute top-4 right-4 text-gray-400 hover:text-gray-600 transition-colors p-2 text-2xl"
                aria-label="Close"
            >
                "✕"
            </button>
            {move || {
                if loading.get() {
                    return view! { <div class="p-8 text-center">"Loading message..."</div> }.into_view();
                }
                
                match email.get() {
                    Some(e) => {
                        let sender_char = e.sender.chars().next().unwrap_or('?').to_uppercase().to_string();
                        let body_html = e.body_html.clone();
                        
                        view! {
                            <>
                            <h1 class="text-2xl font-normal text-gray-900 mb-4 pr-10">{e.subject}</h1>
                            <div class="flex items-start justify-between mb-6">
                                <div class="flex gap-3">
                                    <div class="w-10 h-10 rounded-full bg-blue-600 flex items-center justify-center text-white font-bold text-lg">
                                        {sender_char}
                                    </div>
                                    <div>
                                        <div class="font-bold text-gray-900">{e.sender}</div>
                                        <div class="text-sm text-gray-500">"to " {e.to}</div>
                                    </div>
                                </div>
                                <div class="text-sm text-gray-500">
                                    {e.date}
                                </div>
                            </div>
                            
                            {if !e.labels.is_empty() {
                                view! {
                                    <div class="flex flex-wrap gap-2 mb-6">
                                        <For
                                            each=move || e.labels.clone()
                                            key=|l| l.clone()
                                            children=move |label| {
                                                let l = label.clone();
                                                view! {
                                                    <button
                                                        on:click=move |_| on_select_label.call(l.clone())
                                                        class="px-2 py-0.5 bg-gray-100 text-gray-600 rounded text-xs font-medium border hover:bg-gray-200 hover:text-gray-800 transition-colors"
                                                    >
                                                        {label}
                                                    </button>
                                                }
                                            }
                                        />
                                    </div>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }}
                            
                            <div class="border-t pt-6 mb-8" inner_html=body_html />
                            
                            {if !e.attachments.is_empty() {
                                view! {
                                    <div class="border-t pt-6">
                                        <h3 class="text-lg font-medium text-gray-900 mb-4 flex items-center gap-2">
                                            <span>"📎"</span> "Attachments (" {e.attachments.len()} ")"
                                        </h3>
                                        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                                            <For
                                                each=move || e.attachments.clone()
                                                key=|att| att.path.clone()
                                                children=move |att| {
                                                    let size_kb = (att.size as f64 / 1024.0);
                                                    let size_str = format!("{:.1} KB", size_kb);
                                                    let download_url = format!("http://localhost:8001/attachment/{}", att.path);
                                                    view! {
                                                        <div class="border rounded-lg p-3 flex flex-col gap-2 hover:bg-gray-50 transition-colors">
                                                            <div class="font-medium text-gray-800 truncate" title=att.filename.clone()>
                                                                {att.filename.clone()}
                                                            </div>
                                                            <div class="text-xs text-gray-500">
                                                                {size_str}
                                                            </div>
                                                            <a
                                                                href=download_url
                                                                class="mt-2 text-blue-600 hover:text-blue-800 text-sm font-medium flex items-center gap-1"
                                                                download=att.filename
                                                                target="_blank"
                                                                rel="noopener noreferrer"
                                                            >
                                                                <span>"⬇️"</span> "Download"
                                                            </a>
                                                        </div>
                                                    }
                                                }
                                            />
                                        </div>
                                    </div>
                                }.into_view()
                            } else {
                                view! {}.into_view()
                            }}
                            </>
                        }.into_view()
                    },
                    None => view! { <div class="p-8 text-center text-gray-500">"Select an email to view"</div> }.into_view()
                }
            }}
        </div>
    }
}
