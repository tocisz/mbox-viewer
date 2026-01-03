use leptos::*;

#[component]
pub fn ShortcutHelp(
    #[prop(into)] is_open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <div class=move || {
            let base = "fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm transition-opacity duration-200";
            if is_open.get() {
                format!("{} opacity-100 pointer-events-auto", base)
            } else {
                format!("{} opacity-0 pointer-events-none", base)
            }
        }
        on:click=move |_| on_close.call(())
        >
            <div
                class="bg-white rounded-xl shadow-2xl w-full max-w-4xl max-h-[85vh] overflow-hidden flex flex-col"
                on:click=move |ev| ev.stop_propagation() // Prevent closing when clicking inside modal
            >
                <div class="px-6 py-4 border-b flex items-center justify-between bg-gray-50">
                    <h2 class="text-xl font-bold text-gray-800">"Keyboard Shortcuts"</h2>
                    <button
                        class="text-gray-500 hover:text-gray-700 p-1 rounded-full hover:bg-gray-200 transition"
                        on:click=move |_| on_close.call(())
                    >
                        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                        </svg>
                    </button>
                </div>

                <div class="overflow-y-auto p-6 grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8 text-sm">

                    // Navigation
                    <section class="space-y-3">
                         <h3 class="font-bold text-gray-900 border-b pb-1">"Navigation"</h3>
                         <div class="grid grid-cols-[1fr,auto] gap-x-2">
                            <span>"Go to Inbox"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then i"</kbd>
                            <span>"Go to Starred"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then s"</kbd>
                            <span>"Go to Sent"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then t"</kbd>
                            <span>"Go to Important"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then m"</kbd>
                            <span>"Go to Trash"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then #"</kbd>
                            <span>"Go to Label"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then l"</kbd>
                            <span>"Go to All Mail"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then a"</kbd>
                            <span>"Next Page (+20)"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then n"</kbd>
                            <span>"Prev Page (-20)"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"g then p"</kbd>
                         </div>
                    </section>

                      // Thread
                    <section class="space-y-3">
                         <h3 class="font-bold text-gray-900 border-b pb-1">"Thread List"</h3>
                         <div class="grid grid-cols-[1fr,auto] gap-x-2">
                            <span>"Newer Thread"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"k"</kbd>
                            <span>"Older Thread"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"j"</kbd>
                            <span>"Open Thread"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"o / Enter"</kbd>
                            <span>"Back to List"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"u"</kbd>
                         </div>
                    </section>

                    // Application
                    <section class="space-y-3">
                         <h3 class="font-bold text-gray-900 border-b pb-1">"Application"</h3>
                         <div class="grid grid-cols-[1fr,auto] gap-x-2">
                            <span>"Search"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"/"</kbd>
                            <span>"Show Shortcuts"</span> <kbd class="font-mono bg-gray-100 px-1 rounded border">"?"</kbd>
                         </div>
                    </section>
                </div>

                <div class="bg-gray-50 px-6 py-3 text-xs text-gray-500 text-center border-t">
                    "Press Esc to close"
                </div>
            </div>
        </div>
    }
}
