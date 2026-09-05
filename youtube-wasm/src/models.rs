use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub struct StorageRequest {
    pub provider_id: String,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub cover_art_url: Option<String>,
    pub stream_url: Option<String>,
    pub quality_hint: Option<String>,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub plays: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedTrack {
    pub stream_url: String,
    pub quality_hint: Option<String>,
    pub duration_ms: Option<u64>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderModule {
    pub id: String,
    pub name: String,
    pub layout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlbumItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<String>,
    pub cover_art_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaylistItem {
    pub id: String,
    pub title: String,
    pub author: Option<String>,
    pub item_count: Option<u32>,
    pub cover_art_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenreItem {
    pub id: String,
    pub title: String,
    pub endpoint_params: Option<String>,
    pub color_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtistItem {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub subscribers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopResultItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub item_type: String, // "artist", "album", "song"
    pub cover_art_url: Option<String>,
    pub provider_id: Option<String>,
    pub provider_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum SearchItem {
    TopResult(TopResultItem),
    Track(TrackResult),
    Album(AlbumItem),
    Artist(ArtistItem),
    Playlist(PlaylistItem),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchCategorySection {
    pub category: String, // "Top Result", "Songs", "Albums", "Artists", "Playlists"
    pub items: Vec<SearchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategorizedSearchResult {
    pub sections: Vec<SearchCategorySection>,
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchQueryInput {
    pub query: String,
    pub filter: Option<String>, // "all", "songs", "albums", "artists", "playlists"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorialSpotlight {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub cover_art_url: Option<String>,
    pub description: Option<String>,
    pub release_year: Option<String>,
    pub track_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryShelf {
    pub title: String,
    pub items: Vec<PlaylistItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum ModuleItem {
    Track(TrackResult),
    Album(AlbumItem),
    Playlist(PlaylistItem),
    Genre(GenreItem),
    Artist(ArtistItem),
    Spotlight(EditorialSpotlight),
    Shelf(CategoryShelf),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleData {
    pub items: Vec<ModuleItem>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumDetailResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<String>,
    pub description: Option<String>,
    pub cover_art_url: Option<String>,
    pub track_count: Option<u32>,
    pub tracks: Vec<TrackResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistDetailResult {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub banner_url: Option<String>,
    pub subscribers: Option<String>,
    pub bio: Option<String>,
    pub top_tracks: Vec<TrackResult>,
    pub albums: Vec<AlbumItem>,
    pub singles: Vec<AlbumItem>,
    #[serde(default)]
    pub videos: Vec<TrackResult>,
    #[serde(default)]
    pub featured_on: Vec<PlaylistItem>,
    #[serde(default)]
    pub similar_artists: Vec<ArtistItem>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalSeedV1 {
    pub abi_version: u32,
    pub canonical_key: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub isrc: Option<String>,
    pub duration_ms: Option<u64>,
    pub native_id: Option<String>,
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RadioStreamResultV1 {
    pub tracks: Vec<TrackResult>,
    pub continuation_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackTelemetryEventV1 {
    pub native_track_id: String,
    pub duration_ms: u64,
    pub total_track_duration_ms: u64,
    pub completed: bool,
}


pub fn is_junk_title(title: &str) -> bool {
    let lower = title.to_lowercase();
    let junk_patterns = [
        "slowed + reverb",
        "slowed & reverb",
        "slowed and reverb",
        "slowed reverb",
        "sped up",
        "speed up",
        "8d audio",
        "bass boosted",
        "ringtone",
        "soundboard",
        "nightcore",
        "tiktok version",
        "instrumental cover",
        "karaoke version",
        "dj remix",
        "dj rahul",
        "dj mix",
        "status video",
        "female version",
        "male version",
        "whatsapp status",
    ];
    junk_patterns.iter().any(|p| lower.contains(p))
}

pub fn is_authentic_album(item: &AlbumItem) -> bool {
    if item.id.trim().is_empty() || item.title.trim().is_empty() || item.artist.trim().is_empty() {
        return false;
    }
    if is_junk_title(&item.title) || is_junk_title(&item.artist) {
        return false;
    }
    if item.artist.eq_ignore_ascii_case("unknown artist") {
        return false;
    }
    true
}

pub fn sanitize_track(mut t: TrackResult) -> Option<TrackResult> {
    if t.id.trim().is_empty() {
        return None;
    }
    if is_junk_title(&t.title) {
        return None;
    }
    let (cleaned_title, cleaned_artist) = clean_title_and_artist(&t.title, &t.artist);
    if cleaned_title.is_empty() || cleaned_artist.is_empty() {
        return None;
    }
    t.title = cleaned_title;
    t.artist = cleaned_artist;
    Some(t)
}

pub fn sanitize_module_items(items: Vec<ModuleItem>) -> Vec<ModuleItem> {
    let mut clean_items = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for item in items {
        match item {
            ModuleItem::Album(alb) => {
                if !is_authentic_album(&alb) {
                    continue;
                }
                if seen_ids.insert(alb.id.clone()) {
                    clean_items.push(ModuleItem::Album(alb));
                }
            }
            ModuleItem::Track(trk) => {
                if let Some(clean_trk) = sanitize_track(trk) {
                    if seen_ids.insert(clean_trk.id.clone()) {
                        clean_items.push(ModuleItem::Track(clean_trk));
                    }
                }
            }
            ModuleItem::Spotlight(spot) => {
                if spot.id.trim().is_empty() || spot.title.trim().is_empty() || is_junk_title(&spot.title) {
                    continue;
                }
                if seen_ids.insert(spot.id.clone()) {
                    clean_items.push(ModuleItem::Spotlight(spot));
                }
            }
            ModuleItem::Genre(genre) => {
                if genre.title.trim().is_empty() {
                    continue;
                }
                let dedupe_key = format!("{}:{}", genre.title.to_lowercase(), genre.endpoint_params.as_deref().unwrap_or(&genre.id));
                if seen_ids.insert(dedupe_key) {
                    clean_items.push(ModuleItem::Genre(genre));
                }
            }
            ModuleItem::Artist(artist) => {
                if artist.id.trim().is_empty() || artist.name.trim().is_empty() {
                    continue;
                }
                if seen_ids.insert(artist.id.clone()) {
                    clean_items.push(ModuleItem::Artist(artist));
                }
            }
            ModuleItem::Playlist(pl) => {
                if pl.id.trim().is_empty() || pl.title.trim().is_empty() || is_junk_title(&pl.title) {
                    continue;
                }
                if seen_ids.insert(pl.id.clone()) {
                    clean_items.push(ModuleItem::Playlist(pl));
                }
            }
            ModuleItem::Shelf(shelf) => {
                if !shelf.items.is_empty() {
                    clean_items.push(ModuleItem::Shelf(shelf));
                }
            }
        }
    }
    clean_items
}

pub fn sanitize_track_results(tracks: Vec<TrackResult>) -> Vec<TrackResult> {
    let mut clean_tracks = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for t in tracks {
        if let Some(clean) = sanitize_track(t) {
            if seen_ids.insert(clean.id.clone()) {
                clean_tracks.push(clean);
            }
        }
    }
    clean_tracks
}

pub fn clean_title_and_artist(title: &str, artist: &str) -> (String, String) {
    let mut t = title.trim().to_string();
    let mut a = artist.trim().to_string();

    if a.starts_with('@') {
        a = a.trim_start_matches('@').trim().to_string();
    }

    // Split 'Artist - Title' if title includes hyphen separator
    if let Some(pos) = t.find(" - ") {
        let prefix = &t[..pos].trim();
        let suffix = &t[pos + 3..].trim();
        if prefix.starts_with('@') || prefix.eq_ignore_ascii_case(&a) || a.is_empty() {
            if a.is_empty() {
                a = prefix.trim_start_matches('@').trim().to_string();
            }
            t = suffix.to_string();
        }
    }

    // Strip pipe metadata: 'Song | Movie | Actor | Singer' -> 'Song'
    if let Some(pos) = t.find(" | ") {
        let first_part = t[..pos].trim();
        let rest = &t[pos + 3..];
        
        let label_channels = ["t-series", "saregama", "think music", "sony", "tips", "zee", "speed records", "yrf"];
        let is_label = label_channels.iter().any(|lbl| a.to_lowercase().contains(lbl));
        if is_label {
            for segment in rest.split(" | ") {
                let seg_clean = segment.trim();
                let lower_seg = seg_clean.to_lowercase();
                if !lower_seg.contains("video") && !lower_seg.contains("8k") && !lower_seg.contains("4k") && !lower_seg.contains("song") && !lower_seg.contains("movie") && !lower_seg.contains("teaser") {
                    a = seg_clean.to_string();
                    break;
                }
            }
        }
        t = first_part.to_string();
    }

    // Strip common YouTube junk tags
    let junk_tags = [
        "(Official Music Video)", "(Official Video)", "(Official Audio)", "[Official Music Video]",
        "[Official Video]", "[Official Audio]", "(Music Video)", "[Music Video]",
        "(Full Video Song)", "(Full Song)", "(Audio)", "[Audio]", "(Lyric Video)", "[Lyric Video]",
        "(Lyrics)", "[Lyrics]", "(Visualizer)", "[Visualizer]", "(Live)", "(Acoustic)", "(Remix)",
        "Official MV", "Official Video", "8K VIDEO", "4K VIDEO", "HD VIDEO", "8K Video", "4K Video",
        "(Official Track Video)", "[Official Track Video]"
    ];
    for tag in &junk_tags {
        t = t.replace(tag, "");
    }

    // Trim trailing quotes, hyphens, and whitespace
    t = t.trim().trim_matches(|c| c == '"' || c == '\'' || c == '-' || c == '|').trim().to_string();

    (t, a)
}


pub fn clean_seed_artist_and_title(artist: &str, title: &str) -> (String, String) {
    let mut clean_artist = artist.trim().to_string();
    if let Some(first) = clean_artist.split(',').next() {
        clean_artist = first.trim().to_string();
    }
    if let Some(first) = clean_artist.split('&').next() {
        clean_artist = first.trim().to_string();
    }
    if let Some(first) = clean_artist.split(" feat").next() {
        clean_artist = first.trim().to_string();
    }

    let mut clean_title = title.trim().to_string();
    if let Some(pos) = clean_title.find(" - ") {
        clean_title = clean_title[pos + 3..].trim().to_string();
    }
    for tag in &["(Official Music Video)", "(Official Video)", "(Official Audio)", "[Audio]", "(Audio)", "(Lyric Video)", "(Lyrics)", "(Live)", "(Acoustic)", "(Remix)"] {
        clean_title = clean_title.replace(tag, "").trim().to_string();
    }

    (clean_artist, clean_title)
}

