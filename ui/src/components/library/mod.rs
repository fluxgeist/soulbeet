use api::{consolidate_album, delete_library_album, get_library};
use dioxus::prelude::*;
use shared::library::AlbumEntry;

use crate::auth::use_auth;

#[derive(Clone, PartialEq)]
enum SortMode {
    Artist,
    Album,
    DateAdded,
}

#[component]
pub fn Library() -> Element {
    let mut albums = use_signal(Vec::<AlbumEntry>::new);
    let mut error = use_signal(|| "".to_string());
    let mut deleting = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<AlbumEntry>);
    let mut consolidating = use_signal(|| None::<String>);
    let mut sort_mode = use_signal(|| SortMode::Artist);
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

    let handle_consolidate = move |entry: AlbumEntry| async move {
        let key = format!("{}/{}", entry.artist, entry.album);
        consolidating.set(Some(key));
        error.set("".to_string());

        match auth
            .call(consolidate_album(entry.artist.clone(), entry.album.clone()))
            .await
        {
            Ok(_) => {
                fetch_library().await;
            }
            Err(e) => error.set(format!("Failed to consolidate: {e}")),
        }

        consolidating.set(None);
    };

    let mut sorted = albums.read().clone();
    match *sort_mode.read() {
        SortMode::Artist => sorted.sort_by(|a, b| {
            a.artist
                .to_lowercase()
                .cmp(&b.artist.to_lowercase())
                .then(a.album.to_lowercase().cmp(&b.album.to_lowercase()))
        }),
        SortMode::Album => sorted.sort_by(|a, b| {
            a.album
                .to_lowercase()
                .cmp(&b.album.to_lowercase())
                .then(a.artist.to_lowercase().cmp(&b.artist.to_lowercase()))
        }),
        SortMode::DateAdded => sorted.sort_by(|a, b| {
            b.added
                .partial_cmp(&a.added)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    // Sequential grouping preserves sort order (only used in Artist mode)
    let grouped: Vec<(String, Vec<AlbumEntry>)> =
        sorted.iter().fold(vec![], |mut acc, album| {
            if let Some(last) = acc.last_mut() {
                if last.0 == album.artist {
                    last.1.push(album.clone());
                    return acc;
                }
            }
            acc.push((album.artist.clone(), vec![album.clone()]));
            acc
        });

    let is_artist = *sort_mode.read() == SortMode::Artist;
    let is_album = *sort_mode.read() == SortMode::Album;
    let is_date = *sort_mode.read() == SortMode::DateAdded;

    rsx! {
        div { class: "max-w-4xl mx-auto",
            div { class: "flex flex-wrap items-center justify-between gap-3 mb-6",
                h1 { class: "text-2xl font-bold text-beet-accent font-display", "Library" }
                div { class: "flex items-center gap-2",
                    span { class: "text-xs text-gray-500 font-mono hidden sm:inline", "Sort by" }
                    button {
                        class: if is_artist {
                            "px-3 py-1 rounded text-xs font-mono text-beet-accent bg-white/10 border border-white/20 cursor-pointer transition-colors"
                        } else {
                            "px-3 py-1 rounded text-xs font-mono text-gray-400 bg-white/5 border border-transparent hover:text-white hover:bg-white/10 cursor-pointer transition-colors"
                        },
                        onclick: move |_| sort_mode.set(SortMode::Artist),
                        "Artist"
                    }
                    button {
                        class: if is_album {
                            "px-3 py-1 rounded text-xs font-mono text-beet-accent bg-white/10 border border-white/20 cursor-pointer transition-colors"
                        } else {
                            "px-3 py-1 rounded text-xs font-mono text-gray-400 bg-white/5 border border-transparent hover:text-white hover:bg-white/10 cursor-pointer transition-colors"
                        },
                        onclick: move |_| sort_mode.set(SortMode::Album),
                        "Album"
                    }
                    button {
                        class: if is_date {
                            "px-3 py-1 rounded text-xs font-mono text-beet-accent bg-white/10 border border-white/20 cursor-pointer transition-colors"
                        } else {
                            "px-3 py-1 rounded text-xs font-mono text-gray-400 bg-white/5 border border-transparent hover:text-white hover:bg-white/10 cursor-pointer transition-colors"
                        },
                        onclick: move |_| sort_mode.set(SortMode::DateAdded),
                        "Date Added"
                    }
                }
            }

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

            if sorted.is_empty() {
                div { class: "text-center py-20 text-gray-500 font-mono",
                    p { class: "text-lg mb-2", "No albums in library" }
                    p { class: "text-sm", "Download some music from the Search tab to get started." }
                }
            } else if is_artist {
                // Grouped by artist
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
                                            artist_albums.iter().map(|entry| {
                                                let entry_for_delete = entry.clone();
                                                let entry_for_consolidate = entry.clone();
                                                let key = format!("{}/{}", entry.artist, entry.album);
                                                let is_deleting = deleting.read().as_deref() == Some(&key);
                                                let is_consolidating = consolidating.read().as_deref() == Some(&key);
                                                let track_label = if entry.track_count == 1 { "track" } else { "tracks" };
                                                rsx! {
                                                    div {
                                                        key: "{key}",
                                                        class: "flex items-center justify-between p-3 bg-white/5 rounded hover:bg-white/8 transition-colors group",
                                                        div { class: "min-w-0",
                                                            span { class: "text-white font-medium block truncate", "{entry.album}" }
                                                            span { class: "text-gray-500 text-xs font-mono",
                                                                "{entry.track_count} {track_label}"
                                                            }
                                                        }
                                                        div { class: "flex items-center gap-3 ml-4 shrink-0 sm:opacity-0 sm:group-hover:opacity-100 transition-all",
                                                            button {
                                                                class: "text-xs font-mono text-gray-500 hover:text-beet-accent underline decoration-dotted",
                                                                disabled: is_consolidating || is_deleting,
                                                                onclick: move |_| handle_consolidate(entry_for_consolidate.clone()),
                                                                if is_consolidating { "Fixing..." } else { "Fix in Navidrome" }
                                                            }
                                                            button {
                                                                class: "text-xs font-mono text-gray-500 hover:text-red-400 underline decoration-dotted",
                                                                disabled: is_deleting || is_consolidating,
                                                                onclick: move |_| confirm_delete.set(Some(entry_for_delete.clone())),
                                                                if is_deleting { "Deleting..." } else { "Delete" }
                                                            }
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
            } else {
                // Flat list (Album or Date Added sort)
                div { class: "space-y-1",
                    {
                        sorted.iter().map(|entry| {
                            let entry_for_delete = entry.clone();
                            let entry_for_consolidate = entry.clone();
                            let key = format!("{}/{}", entry.artist, entry.album);
                            let is_deleting = deleting.read().as_deref() == Some(&key);
                            let is_consolidating = consolidating.read().as_deref() == Some(&key);
                            let track_label = if entry.track_count == 1 { "track" } else { "tracks" };
                            rsx! {
                                div {
                                    key: "{key}",
                                    class: "flex items-center justify-between p-3 bg-white/5 rounded hover:bg-white/8 transition-colors group",
                                    div { class: "min-w-0",
                                        span { class: "text-white font-medium block truncate", "{entry.album}" }
                                        span { class: "text-gray-500 text-xs font-mono",
                                            "{entry.artist} · {entry.track_count} {track_label}"
                                        }
                                    }
                                    div { class: "flex items-center gap-3 ml-4 shrink-0 sm:opacity-0 sm:group-hover:opacity-100 transition-all",
                                        button {
                                            class: "text-xs font-mono text-gray-500 hover:text-beet-accent underline decoration-dotted",
                                            disabled: is_consolidating || is_deleting,
                                            onclick: move |_| handle_consolidate(entry_for_consolidate.clone()),
                                            if is_consolidating { "Fixing..." } else { "Fix in Navidrome" }
                                        }
                                        button {
                                            class: "text-xs font-mono text-gray-500 hover:text-red-400 underline decoration-dotted",
                                            disabled: is_deleting || is_consolidating,
                                            onclick: move |_| confirm_delete.set(Some(entry_for_delete.clone())),
                                            if is_deleting { "Deleting..." } else { "Delete" }
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
