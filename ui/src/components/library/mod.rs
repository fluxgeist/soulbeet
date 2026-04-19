use api::{delete_library_album, get_library};
use dioxus::prelude::*;
use shared::library::AlbumEntry;

use crate::auth::use_auth;

#[component]
pub fn Library() -> Element {
    let mut albums = use_signal(Vec::<AlbumEntry>::new);
    let mut error = use_signal(|| "".to_string());
    let mut deleting = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<AlbumEntry>);
    let auth = use_auth();

    let fetch_library = move || async move {
        match auth.call(get_library()).await {
            Ok(data) => albums.set(data),
            Err(e) => error.set(format!("Failed to load library: {e}")),
        }
    };

    use_future(move || async move {
        fetch_library().await;
    });

    // Group albums by artist
    let grouped: Vec<(String, Vec<AlbumEntry>)> = {
        let mut map: std::collections::HashMap<String, Vec<AlbumEntry>> = std::collections::HashMap::new();
        for album in albums.read().iter() {
            map.entry(album.artist.clone()).or_default().push(album.clone());
        }
        let mut entries: Vec<(String, Vec<AlbumEntry>)> = map.into_iter().collect();
        entries.sort_by(|(a, _), (b, _)| a.to_lowercase().cmp(&b.to_lowercase()));
        entries
    };

    let handle_delete = move |entry: AlbumEntry| async move {
        let key = format!("{}/{}", entry.artist, entry.album);
        deleting.set(Some(key.clone()));
        error.set("".to_string());

        match auth
            .call(delete_library_album(
                entry.album_path.clone(),
                entry.artist.clone(),
                entry.album.clone(),
            ))
            .await
        {
            Ok(_) => {
                fetch_library().await;
            }
            Err(e) => error.set(format!("Failed to delete: {e}")),
        }

        deleting.set(None);
        confirm_delete.set(None);
    };

    rsx! {
        div { class: "max-w-4xl mx-auto",
            h1 { class: "text-2xl font-bold text-beet-accent font-display mb-6", "Library" }

            if !error().is_empty() {
                div { class: "mb-4 p-4 bg-red-900/20 border border-red-500/50 rounded text-red-400 font-mono text-sm",
                    "{error}"
                }
            }

            // Confirm delete modal
            if let Some(entry) = confirm_delete.read().clone() {
                div { class: "fixed inset-0 bg-black/70 flex items-center justify-center z-50",
                    div { class: "bg-beet-panel border border-white/10 rounded-lg p-6 max-w-sm w-full mx-4 shadow-2xl",
                        h3 { class: "text-lg font-bold text-white font-display mb-2", "Delete Album?" }
                        p { class: "text-gray-400 font-mono text-sm mb-1",
                            span { class: "text-white", "{entry.artist}" }
                            " — "
                            span { class: "text-beet-accent", "{entry.album}" }
                        }
                        p { class: "text-gray-500 font-mono text-xs mb-6",
                            "This will permanently delete all files and remove the album from your library."
                        }
                        div { class: "flex gap-3",
                            button {
                                class: "flex-1 py-2 px-4 bg-red-600 hover:bg-red-500 text-white rounded font-mono text-sm font-bold transition-colors",
                                onclick: move |_| {
                                    let e = entry.clone();
                                    handle_delete(e)
                                },
                                if deleting.read().is_some() { "Deleting..." } else { "Delete" }
                            }
                            button {
                                class: "flex-1 py-2 px-4 bg-white/5 hover:bg-white/10 text-gray-300 rounded font-mono text-sm transition-colors",
                                onclick: move |_| confirm_delete.set(None),
                                "Cancel"
                            }
                        }
                    }
                }
            }

            if albums.read().is_empty() {
                div { class: "text-center py-20 text-gray-500 font-mono",
                    p { class: "text-lg mb-2", "No albums in library" }
                    p { class: "text-sm", "Download some music from the Search tab to get started." }
                }
            } else {
                div { class: "space-y-6",
                    {
                        grouped.iter().map(|(artist, artist_albums)| {
                            rsx! {
                                div { key: "{artist}",
                                    h2 { class: "text-sm font-mono text-gray-400 uppercase tracking-widest mb-2 border-b border-white/5 pb-1",
                                        "{artist}"
                                    }
                                    div { class: "space-y-1",
                                        {
                                            artist_albums.into_iter().map(|entry| {
                                                let entry_for_delete = entry.clone();
                                                let key = format!("{}/{}", entry.artist, entry.album);
                                                let is_deleting = deleting.read().as_deref() == Some(&key);
                                                let track_label = if entry.track_count == 1 { "track" } else { "tracks" };
                                                rsx! {
                                                    div {
                                                        key: "{key}",
                                                        class: "flex items-center justify-between p-3 bg-white/5 rounded hover:bg-white/8 transition-colors group",
                                                        div { class: "flex items-center gap-3 min-w-0",
                                                            div { class: "min-w-0",
                                                                span { class: "text-white font-medium block truncate", "{entry.album}" }
                                                                span { class: "text-gray-500 text-xs font-mono",
                                                                    "{entry.track_count} {track_label}"
                                                                }
                                                            }
                                                        }
                                                        button {
                                                            class: "opacity-0 group-hover:opacity-100 text-xs font-mono text-gray-500 hover:text-red-400 transition-all underline decoration-dotted ml-4 shrink-0",
                                                            disabled: is_deleting,
                                                            onclick: move |_| confirm_delete.set(Some(entry_for_delete.clone())),
                                                            if is_deleting { "Deleting..." } else { "Delete" }
                                                        }
                                                    }
                                                }
                                            })
                                        }
                                    }
                                }
                            }
                        })
                    }
                }
            }
        }
    }
}
