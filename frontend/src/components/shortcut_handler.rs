use leptos::*;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

#[derive(Clone, Debug, PartialEq)]
pub enum ShortcutAction {
    Help,
    CloseHelp,
    Search,
    // Navigation
    GoToInbox,
    GoToSent,
    GoToDrafts,
    GoToAll,
    GoToStarred,
    GoToTrash,
    GoToImportant,
    GoToLabel,

    // Thread
    NextThread,
    PrevThread,
    OpenThread,
    BackToList,
    GoToNextPage,
    GoToPrevPage,
}

#[component]
pub fn ShortcutHandler(#[prop(into)] on_action: Callback<ShortcutAction>) -> impl IntoView {
    let (key_sequence, set_key_sequence) = create_signal(String::new());

    // We attach listener to window
    let handle_keydown = window_event_listener(ev::keydown, move |ev: KeyboardEvent| {
        let key = ev.key();
        let target = ev.target();

        // Ignore shortcuts if an input/textarea is focused, except for Escape
        if let Some(target) = target.as_ref() {
            let is_input = target.dyn_ref::<web_sys::HtmlInputElement>().is_some()
                || target.dyn_ref::<web_sys::HtmlTextAreaElement>().is_some();
            if is_input && key != "Escape" && key != "Esc" {
                return;
            }
        }

        // Escape Handling
        if key == "Escape" || key == "Esc" {
            on_action.call(ShortcutAction::CloseHelp);
            // Also clear sequence
            set_key_sequence.set(String::new());
            if let Some(target) = ev.target() {
                if let Some(el) = target.dyn_ref::<web_sys::HtmlInputElement>() {
                    let _ = el.blur();
                }
            }
            return;
        }

        // Handle Global Shortcuts that don't depend on sequence
        if ev.key() == "?" {
            // Shift + / usually
            on_action.call(ShortcutAction::Help);
            return;
        }

        if ev.key() == "/" {
            // Prevent default / type in some browsers if it's quick search
            ev.prevent_default();
            on_action.call(ShortcutAction::Search);
            return;
        }

        // Sequence handling
        let current_seq = key_sequence.get();

        if current_seq.is_empty() {
            match key.as_str() {
                "g" => {
                    set_key_sequence.set(key.clone());
                }

                // Single key actions
                "u" => {
                    on_action.call(ShortcutAction::BackToList);
                }
                "j" => {
                    on_action.call(ShortcutAction::NextThread);
                }
                "k" => {
                    on_action.call(ShortcutAction::PrevThread);
                }
                "o" | "Enter" => {
                    on_action.call(ShortcutAction::OpenThread);
                }

                _ => {}
            }
        } else {
            // We have a start key (g only now)
            let full_seq = format!("{}{}", current_seq, key);

            match full_seq.as_str() {
                // Navigation g + ...
                "gi" => on_action.call(ShortcutAction::GoToInbox),
                "gt" => on_action.call(ShortcutAction::GoToSent),
                "gd" => on_action.call(ShortcutAction::GoToDrafts),
                "ga" => on_action.call(ShortcutAction::GoToAll),
                "gs" => on_action.call(ShortcutAction::GoToStarred),
                "g#" => on_action.call(ShortcutAction::GoToTrash),
                "gm" => on_action.call(ShortcutAction::GoToImportant),
                "gl" => on_action.call(ShortcutAction::GoToLabel),
                "gn" => on_action.call(ShortcutAction::GoToNextPage),
                "gp" => on_action.call(ShortcutAction::GoToPrevPage),

                _ => {}
            }

            // Always clear sequence after 2nd key (or if invalid)
            set_key_sequence.set(String::new());
        }
    });

    on_cleanup(move || handle_keydown.remove());

    view! {}
}
