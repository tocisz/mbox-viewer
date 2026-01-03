use leptos::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

mod components;
use components::email_detail::EmailDetail;
use components::email_list::EmailList;
use components::sidebar::Sidebar;

use components::label_selector::LabelSelector;
use components::shortcut_handler::{ShortcutAction, ShortcutHandler};
use components::shortcut_help::ShortcutHelp;

// Models
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub subject: String,
    pub sender: String,
    pub date: String,
    pub snippet: String,
    pub labels: Vec<String>,
    pub has_attachment: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDetailData {
    pub id: String,
    pub subject: String,
    pub sender: String,
    pub to: String,
    pub date: String,
    pub labels: Vec<String>,
    pub body_html: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub size: usize,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResponse {
    pub total: usize,
    pub page: usize,
    pub size: usize,
    pub items: Vec<Email>,
}

#[component]
pub fn App() -> impl IntoView {
    // Global State
    let (selected_label, set_selected_label) = create_signal("Inbox".to_string());
    let (search_query, set_search_query) = create_signal("".to_string());
    let (selected_email_id, set_selected_email_id) = create_signal::<Option<String>>(None);
    let (start_date, set_start_date) = create_signal("".to_string());
    let (end_date, set_end_date) = create_signal("".to_string());

    // Shortcut State
    let (is_help_open, set_is_help_open) = create_signal(false);
    let (is_label_selector_open, set_is_label_selector_open) = create_signal(false);
    let (last_shortcut, set_last_shortcut) = create_signal::<Option<ShortcutAction>>(None);

    // Callbacks
    let handle_select_label = move |label: String| {
        set_selected_label.set(label);
        set_search_query.set("".to_string());
        set_selected_email_id.set(None);
    };

    let handle_select_email = move |id: String| {
        set_selected_email_id.set(Some(id));
    };

    let handle_close_detail = move |_| {
        set_selected_email_id.set(None);
    };

    // Shortcut Handler logic
    let on_shortcut = move |action: ShortcutAction| {
        // Handle Global Actions immediately
        match action {
            ShortcutAction::Help => set_is_help_open.update(|v| *v = !*v),
            ShortcutAction::CloseHelp => set_is_help_open.set(false),
            ShortcutAction::Search => {
                // Focus search logic is tricky without a ref,
                // but we can pass this down to Header or handle via native DOM if desperate.
                // For now, let's update a signal maybe? Or try to select the input.
                if let Some(doc) = document().dyn_into::<web_sys::Document>().ok() {
                    if let Some(el) = doc.get_element_by_id("search-input") {
                        let _ = el.dyn_into::<web_sys::HtmlElement>().map(|h| h.focus());
                    }
                }
            }

            ShortcutAction::BackToList => {
                set_selected_email_id.set(None);
            }

            // Navigation
            ShortcutAction::GoToInbox => handle_select_label("Inbox".to_string()),
            ShortcutAction::GoToSent => handle_select_label("Sent".to_string()),
            ShortcutAction::GoToDrafts => handle_select_label("Drafts".to_string()),
            ShortcutAction::GoToAll => handle_select_label("ALL".to_string()),
            ShortcutAction::GoToStarred => handle_select_label("Starred".to_string()),
            ShortcutAction::GoToTrash => handle_select_label("Trash".to_string()),
            ShortcutAction::GoToImportant => handle_select_label("Important".to_string()),
            ShortcutAction::GoToLabel => set_is_label_selector_open.set(true),

            // Pass others to EmailList (j, k, selection etc) via signal
            _ => {
                set_last_shortcut.set(Some(action));
            }
        }
    };

    view! {
        <div class="flex h-screen w-screen flex-col bg-gray-100 text-sm">
            <ShortcutHandler on_action=on_shortcut />
            <ShortcutHelp is_open=is_help_open on_close=move |_| set_is_help_open.set(false) />
            <LabelSelector
                is_open=is_label_selector_open
                on_close=move |_| set_is_label_selector_open.set(false)
                on_select=handle_select_label
            />

            <Header
                search_query=search_query
                set_search_query=set_search_query
                start_date=start_date
                set_start_date=set_start_date
                end_date=end_date
                set_end_date=set_end_date
                set_selected_label=set_selected_label
            />

            <div class="flex flex-1 overflow-hidden">
                <Sidebar selected_label=selected_label on_select_label=handle_select_label />

                <div class="flex-1 flex overflow-hidden">
                     <div class=move || {
                         let base = "flex flex-col";
                         if selected_email_id.get().is_some() {
                             format!("{} w-1/3 border-r hidden md:flex", base)
                         } else {
                             format!("{} w-full", base)
                         }
                     }>
                        <EmailList
                            label=selected_label
                            query=search_query
                            start_date=start_date
                            end_date=end_date
                            selected_email_id=selected_email_id
                            on_select_email=handle_select_email
                            shortcut_signal=last_shortcut // Pass the signal
                        />
                     </div>

                    {move || {
                        match selected_email_id.get() {
                            Some(id) => view! {
                                <div class="flex-1 flex flex-col overflow-hidden bg-white relative">
                                    <EmailDetail
                                        id=id
                                        on_close=handle_close_detail
                                        on_select_label=handle_select_label
                                    />
                                </div>
                            }.into_view(),
                            None => view! {
                                <div class="hidden md:flex flex-1 items-center justify-center text-gray-400 bg-gray-50 flex-col gap-4">
                                    <div class="text-6xl">"📬"</div>
                                    <div>"Select an email to read"</div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
fn Header(
    search_query: ReadSignal<String>,
    set_search_query: WriteSignal<String>,
    start_date: ReadSignal<String>,
    set_start_date: WriteSignal<String>,
    end_date: ReadSignal<String>,
    set_end_date: WriteSignal<String>,
    set_selected_label: WriteSignal<String>,
) -> impl IntoView {
    let _ = search_query; // Suppress unused warning
    let (input, set_input) = create_signal("".to_string());

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let val = input.get();
        set_search_query.set(val.clone());
        if !val.is_empty() {
            set_selected_label.set("".to_string());
        }
    };

    let clear_dates = move |_| {
        set_start_date.set("".to_string());
        set_end_date.set("".to_string());
    };

    view! {
        <header class="bg-white border-b px-4 py-2 flex items-center justify-between shadow-sm z-10">
            <div class="flex items-center gap-4 w-full">
                <div class="w-64 font-bold text-xl text-gray-700 flex items-center gap-2">
                    <span>"✉️"</span> "ArchiveViewer"
                </div>
                <form on:submit=on_submit class="flex-1 max-w-xl">
                    <div class="relative">
                        <input
                            id="search-input"
                            type="text"
                            class="w-full bg-gray-100 border-none rounded-lg py-2.5 px-4 focus:bg-white focus:shadow shadow-inner transition-all outline-none"
                            placeholder="Search mail"
                            prop:value=input
                            on:input=move |ev| set_input.set(event_target_value(&ev))
                        />
                    </div>
                </form>
                <div class="flex items-center gap-2 text-gray-600">
                    <div class="flex items-center gap-1">
                        <label class="text-xs font-medium">"From:"</label>
                        <input
                            type="date"
                            class="bg-gray-100 border-none rounded-md py-1 px-2 outline-none focus:bg-white focus:ring-1 focus:ring-blue-400"
                            prop:value=start_date
                            on:input=move |ev| set_start_date.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="flex items-center gap-1">
                        <label class="text-xs font-medium">"To:"</label>
                        <input
                            type="date"
                            class="bg-gray-100 border-none rounded-md py-1 px-2 outline-none focus:bg-white focus:ring-1 focus:ring-blue-400"
                            prop:value=end_date
                            on:input=move |ev| set_end_date.set(event_target_value(&ev))
                        />
                    </div>
                    {move || {
                        if !start_date.get().is_empty() || !end_date.get().is_empty() {
                            view! {
                                <button
                                    on:click=clear_dates
                                    class="text-xs text-blue-600 hover:text-blue-800 font-medium"
                                >
                                    "Clear"
                                </button>
                            }.into_view()
                        } else {
                            view! {}.into_view()
                        }
                    }}
                </div>
            </div>
        </header>
    }
}
