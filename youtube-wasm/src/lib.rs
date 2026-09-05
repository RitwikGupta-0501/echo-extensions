

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

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

#[host_fn]
extern "ExtismHost" {
    fn host_execute_webview_js(script: String) -> String;
    fn host_log(msg: String);
    fn host_http_request(req: String) -> String;
    fn host_telemetry_request(req: String) -> String;
    fn host_storage_get(req: String) -> String;
    fn host_storage_set(req: String);
}

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

fn do_http(method: &str, url: &str, headers: Option<HashMap<String, String>>, body: Option<String>) -> FnResult<HttpResponse> {
    let req = HttpRequest {
        method: method.to_string(),
        url: url.to_string(),
        headers,
        body,
    };
    let json_req = serde_json::to_string(&req)?;
    let res_json = unsafe { host_http_request(json_req)? };
    let res: HttpResponse = serde_json::from_str(&res_json)?;
    Ok(res)
}

fn get_storage(key: &str) -> FnResult<String> {
    let req = StorageRequest {
        provider_id: "youtube-wasm".to_string(),
        key: key.to_string(),
        value: None,
    };
    let json = serde_json::to_string(&req)?;
    let res = unsafe { host_storage_get(json)? };
    Ok(res)
}

fn set_storage(key: &str, value: &str) -> FnResult<()> {
    let req = StorageRequest {
        provider_id: "youtube-wasm".to_string(),
        key: key.to_string(),
        value: Some(value.to_string()),
    };
    let json = serde_json::to_string(&req)?;
    unsafe { host_storage_set(json)? };
    Ok(())
}

const STS_CACHE_TTL_SECS: u64 = 6 * 60 * 60; // 6 hours

#[derive(Serialize, Deserialize)]
struct StsCache {
    sts: i64,
    player_hash: String,
    timestamp_secs: u64,
}

fn extract_player_hash(content: &str) -> Option<String> {
    let markers = ["/s/player/", r"\/s\/player\/", "/player/", r"\/player\/"];
    for marker in markers {
        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find(marker) {
            let start = search_from + pos + marker.len();
            let remainder = &content[start..];
            if let Some(end_idx) = remainder.find(|c: char| c == '/' || c == '\\' || c == '"' || c == '\'') {
                let candidate = &remainder[..end_idx];
                if candidate.len() >= 6 && candidate.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                    return Some(candidate.to_string());
                }
            }
            search_from = start;
        }
    }
    None
}

fn extract_signature_timestamp(player_js: &str) -> Option<i64> {
    // 1. Anchored signatureTimestamp search (Tier 1)
    for key in ["signatureTimestamp", "\"signatureTimestamp\"", "'signatureTimestamp'"] {
        let mut search_from = 0;
        while let Some(pos) = player_js[search_from..].find(key) {
            let start = search_from + pos + key.len();
            let remainder = &player_js[start..];
            let mut chars = remainder.char_indices();
            let mut num_start = None;
            while let Some((idx, c)) = chars.next() {
                if c.is_ascii_digit() {
                    num_start = Some(start + idx);
                    break;
                } else if c != ':' && c != '=' && c != ' ' && c != '\t' && c != '"' && c != '\'' {
                    break;
                }
            }
            if let Some(n_start) = num_start {
                let num_str: String = player_js[n_start..].chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(val) = num_str.parse::<i64>() {
                    if val > 10000 {
                        return Some(val);
                    }
                }
            }
            search_from = start;
        }
    }

    // 2. Loose sts search (Tier 2)
    for key in ["sts:", "sts=", "\"sts\":", "'sts':"] {
        let mut search_from = 0;
        while let Some(pos) = player_js[search_from..].find(key) {
            let start = search_from + pos + key.len();
            let remainder = &player_js[start..];
            let mut chars = remainder.char_indices();
            let mut num_start = None;
            while let Some((idx, c)) = chars.next() {
                if c.is_ascii_digit() {
                    num_start = Some(start + idx);
                    break;
                } else if c != ' ' && c != '\t' && c != '"' && c != '\'' {
                    break;
                }
            }
            if let Some(n_start) = num_start {
                let num_str: String = player_js[n_start..].chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(val) = num_str.parse::<i64>() {
                    if val > 10000 {
                        return Some(val);
                    }
                }
            }
            search_from = start;
        }
    }
    None
}

fn fetch_player_hash_from_network() -> Option<String> {
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
    
    // Primary: fetch https://www.youtube.com/iframe_api
    if let Ok(resp) = do_http("GET", "https://www.youtube.com/iframe_api", Some(headers.clone()), None) {
        if resp.status == 200 {
            if let Some(hash) = extract_player_hash(&resp.body) {
                return Some(hash);
            }
        }
    }

    // Fallback: fetch music.youtube.com
    if let Ok(resp) = do_http("GET", "https://music.youtube.com/", Some(headers), None) {
        if resp.status == 200 {
            if let Some(hash) = extract_player_hash(&resp.body) {
                return Some(hash);
            }
        }
    }

    None
}

fn fetch_player_js_and_sts(hash: &str) -> Option<(i64, String)> {
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
    let url = format!("https://www.youtube.com/s/player/{}/player_ias.vflset/en_GB/base.js", hash);

    if let Ok(resp) = do_http("GET", &url, Some(headers), None) {
        if resp.status == 200 {
            let _ = set_storage("yt_player_js", &resp.body);
            let _ = set_storage(&format!("yt_player_js_{}", hash), &resp.body);
            if let Some(sts) = extract_signature_timestamp(&resp.body) {
                return Some((sts, resp.body));
            }
        }
    }
    None
}

fn url_decode_component(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or(""), 16) {
                out.push(val);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn parse_signature_cipher(cipher_str: &str) -> Option<(String, String, String)> {
    let mut s = None;
    let mut sp = "sig".to_string();
    let mut url = None;

    for part in cipher_str.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            match k {
                "s" => s = Some(url_decode_component(v)),
                "sp" => sp = url_decode_component(v),
                "url" => url = Some(url_decode_component(v)),
                _ => {}
            }
        }
    }

    if let (Some(s_val), Some(url_val)) = (s, url) {
        Some((s_val, sp, url_val))
    } else {
        None
    }
}

fn deobfuscate_signature(obfuscated_s: &str, player_hash: &str) -> String {
    if !init_sandbox_player_js_if_needed(player_hash) {
        let _ = unsafe { host_log("[CIPHER] Sandbox not initialized, returning raw signature".to_string()) };
        return obfuscated_s.to_string();
    }

    let escaped_s = obfuscated_s.replace('\\', "\\\\").replace('\'', "\\'");
    let decipher_script = format!(
        r#"(function() {{
            try {{
                if (typeof window.__yt_sig_decipher === 'function') {{
                    var res = window.__yt_sig_decipher('{escaped_s}');
                    if (typeof res === 'string' && res.length >= 10) {{
                        return res;
                    }}
                }}
            }} catch(e) {{}}
            return '{escaped_s}';
        }})()"#
    );

    match unsafe { host_execute_webview_js(decipher_script) } {
        Ok(deciphered) => {
            let trimmed = deciphered.trim().trim_matches('"').to_string();
            if !trimmed.is_empty() && trimmed.len() >= 10 {
                let _ = unsafe { host_log(format!("[CIPHER] Successfully deciphered signature (len: {})", trimmed.len())) };
                trimmed
            } else {
                let _ = unsafe { host_log(format!("[CIPHER] Decipher returned invalid result: {}", trimmed)) };
                obfuscated_s.to_string()
            }
        }
        Err(e) => {
            let _ = unsafe { host_log(format!("[CIPHER] Failed to execute decipher JS: {:?}", e)) };
            obfuscated_s.to_string()
        }
    }
}

fn extract_n_param_from_url(url: &str) -> Option<String> {
    let query = url.split_once('?')?;
    for param in query.1.split('&') {
        if let Some((k, v)) = param.split_once('=') {
            if k == "n" && !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn init_sandbox_player_js_if_needed(player_hash: &str) -> bool {
    let check_js = format!(
        r#"typeof window.__yt_n_transform === 'function' && window.__yt_loaded_player_hash === '{}';"#,
        player_hash
    );
    if let Ok(res) = unsafe { host_execute_webview_js(check_js) } {
        if res.trim() == "true" {
            return true;
        }
    }

    let player_js = get_storage(&format!("yt_player_js_{}", player_hash))
        .or_else(|_| get_storage("yt_player_js"))
        .unwrap_or_default();

    let player_js = if player_js.is_empty() {
        if let Some((_, js)) = fetch_player_js_and_sts(player_hash) {
            js
        } else {
            String::new()
        }
    } else {
        player_js
    };

    if player_js.is_empty() {
        let _ = unsafe { host_log("[N-TRANSFORM] Could not retrieve player.js for init".to_string()) };
        return false;
    }

    let _ = unsafe { host_log(format!("[N-TRANSFORM] Initializing player.js (length: {} chars) in JS Sandbox...", player_js.len())) };

    let player_js_quoted = serde_json::to_string(&player_js).unwrap_or_default(); let init_script = format!(
        r#"(function() {{
            try {{
                if (typeof window.__yt_n_transform === 'function' && window.__yt_loaded_player_hash === '{player_hash}') {{
                    return 'ALREADY_READY';
                }}
                var playerJsStr = {player_js_quoted}; {player_js};
                
                var testInput = "KdrqFlzJXl9EcCwlmEy";
                var nFunc = null;

                var keys = Object.getOwnPropertyNames(window);
                for (var i = 0; i < keys.length; i++) {{
                    var k = keys[i];
                    if (k.startsWith("webkit") || k.startsWith("on") || k.startsWith("__") || k === "window" || k === "self" || k === "bgVm" || k === "bgProgram") continue;
                    try {{
                        var fn = window[k];
                        if (typeof fn === 'function' && fn.length === 1) {{
                            var res = fn(testInput);
                            if (typeof res === 'string' && res !== testInput && res.length >= 5 && /^[a-zA-Z0-9_-]+$/.test(res)) {{
                                nFunc = fn;
                                break;
                            }}
                        }}
                    }} catch(e) {{}}
                }}

                if (nFunc) {{
                    window.__yt_n_transform = nFunc;
                    window.__yt_loaded_player_hash = '{player_hash}';
                    return 'SUCCESS';
                }}
                return 'NO_N_FUNC';
            }} catch(e) {{
                return 'ERROR: ' + e;
            }}
        }})()"#
    );

    match unsafe { host_execute_webview_js(init_script) } {
        Ok(res) => {
            let _ = unsafe { host_log(format!("[N-TRANSFORM] Sandbox init result: {}", res)) };
            res.contains("SUCCESS") || res.contains("ALREADY_READY")
        }
        Err(e) => {
            let _ = unsafe { host_log(format!("[N-TRANSFORM] Sandbox init failed: {:?}", e)) };
            false
        }
    }
}

fn transform_n_param(raw_n: &str, player_hash: &str) -> String {
    if !init_sandbox_player_js_if_needed(player_hash) {
        let _ = unsafe { host_log("[N-TRANSFORM] Sandbox not initialized, returning raw n".to_string()) };
        return raw_n.to_string();
    }

    let transform_script = format!(
        r#"(function() {{
            try {{
                if (typeof window.__yt_n_transform === 'function') {{
                    var res = window.__yt_n_transform('{raw_n}');
                    if (typeof res === 'string' && res.length >= 5 && /^[a-zA-Z0-9_-]+$/.test(res)) {{
                        return res;
                    }}
                }}
            }} catch(e) {{}}
            return '{raw_n}';
        }})()"#
    );

    match unsafe { host_execute_webview_js(transform_script) } {
        Ok(transformed) => {
            let trimmed = transformed.trim().trim_matches('"').to_string();
            if !trimmed.is_empty() && trimmed != raw_n && trimmed.len() >= 5 {
                let _ = unsafe { host_log(format!("[N-TRANSFORM] Successfully transformed n: {} -> {}", raw_n, trimmed)) };
                trimmed
            } else {
                let _ = unsafe { host_log(format!("[N-TRANSFORM] N transform produced identical or invalid result: {}", trimmed)) };
                raw_n.to_string()
            }
        }
        Err(e) => {
            let _ = unsafe { host_log(format!("[N-TRANSFORM] Failed to execute transform JS: {:?}", e)) };
            raw_n.to_string()
        }
    }
}

fn apply_n_transform_to_url(url: &str, player_hash: &str) -> String {
    if let Some(raw_n) = extract_n_param_from_url(url) {
        let transformed_n = transform_n_param(&raw_n, player_hash);
        if transformed_n != raw_n {
            let old_param = format!("n={}", raw_n);
            let new_param = format!("n={}", transformed_n);
            return url.replace(&old_param, &new_param);
        }
    }
    url.to_string()
}


fn get_or_refresh_sts() -> i64 {
    let now_secs = get_unix_timestamp();

    // 1. Check cached STS in host storage
    if let Ok(cached_json) = get_storage("yt_sts_cache") {
        if !cached_json.is_empty() {
            if let Ok(cache) = serde_json::from_str::<StsCache>(&cached_json) {
                if cache.sts > 0 && now_secs >= cache.timestamp_secs && (now_secs - cache.timestamp_secs) < STS_CACHE_TTL_SECS {
                    let _ = unsafe { host_log(format!("[STS] Using cached signatureTimestamp: {} (hash: {}, age: {}s)", cache.sts, cache.player_hash, now_secs - cache.timestamp_secs)) };
                    return cache.sts;
                }
            }
        }
    }

    let _ = unsafe { host_log("[STS] Fetching dynamic player hash from YouTube...".to_string()) };

    // 2. Fetch fresh player hash
    if let Some(hash) = fetch_player_hash_from_network() {
        let _ = unsafe { host_log(format!("[STS] Found active player hash: {}", hash)) };
        if let Some((sts, _player_js)) = fetch_player_js_and_sts(&hash) {
            let _ = unsafe { host_log(format!("[STS] Extracted dynamic signatureTimestamp: {} from base.js (hash: {})", sts, hash)) };
            let cache = StsCache {
                sts,
                player_hash: hash.clone(),
                timestamp_secs: now_secs,
            };
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = set_storage("yt_sts_cache", &json);
                let _ = set_storage("yt_player_hash", &hash);
            }
            return sts;
        } else {
            let _ = unsafe { host_log(format!("[STS] Failed to download base.js or extract STS for hash: {}", hash)) };
        }
    } else {
        let _ = unsafe { host_log("[STS] Failed to extract player hash from iframe_api".to_string()) };
    }

    // 3. Graceful fallback to previously cached STS or default constant
    if let Ok(cached_json) = get_storage("yt_sts_cache") {
        if let Ok(cache) = serde_json::from_str::<StsCache>(&cached_json) {
            if cache.sts > 0 {
                let _ = unsafe { host_log(format!("[STS] Falling back to stale cached signatureTimestamp: {}", cache.sts)) };
                return cache.sts;
            }
        }
    }

    let default_sts = 20111;
    let _ = unsafe { host_log(format!("[STS] Falling back to hardcoded default signatureTimestamp: {}", default_sts)) };
    default_sts
}

const REQUEST_KEY: &str = "O43z0dpjhgX20SCx4KAo";
const GOOGLE_API_KEY: &str = "AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.3";

fn do_botguard_http(url: &str, body: &str) -> FnResult<HttpResponse> {
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());
    headers.insert("Content-Type".to_string(), "application/json+protobuf".to_string());
    headers.insert("x-goog-api-key".to_string(), GOOGLE_API_KEY.to_string());
    headers.insert("x-user-agent".to_string(), "grpc-web-javascript/0.1".to_string());
    
    do_http("POST", url, Some(headers), Some(body.to_string()))
}

fn inject_botguard_functions() -> FnResult<()> {
    let funcs = r#"
        window.bgVmFunctions = null;
        window.bgVm = null;
        window.bgProgram = null;
        window.poTokenMinter = null;

        window.loadBotGuard = function(challengeData) {
            window.bgVm = window[challengeData.globalName];
            window.bgProgram = challengeData.program;
            window.bgVmFunctions = null;
            var vmFunctionsCallback = function (asyncSnapshotFunction, shutdownFunction, passEventFunction, checkCameraFunction) {
                window.bgVmFunctions = { asyncSnapshotFunction: asyncSnapshotFunction };
            };
            window.bgVm.a(window.bgProgram, vmFunctionsCallback, true, undefined, function () {}, [[], []]);
            return new Promise(function (resolve, reject) {
                var attempts = 0;
                var checkInterval = setInterval(function () {
                    if (window.bgVmFunctions && window.bgVmFunctions.asyncSnapshotFunction) {
                        clearInterval(checkInterval);
                        resolve();
                    } else if (attempts >= 10000) {
                        clearInterval(checkInterval);
                        reject(new Error('Timeout waiting for asyncSnapshotFunction'));
                    }
                    attempts++;
                }, 1);
            });
        };

        window.snapshot = function(webPoSignalOutput) {
            return new Promise(function (resolve, reject) {
                window.bgVmFunctions.asyncSnapshotFunction(
                    function (response) { resolve(response); },
                    [undefined, undefined, webPoSignalOutput, undefined]
                );
            });
        };
        "ok";
    "#;
    unsafe { host_log("Executing inject_botguard_functions script".to_string())? };
    unsafe { host_execute_webview_js(funcs.to_string())? };
    unsafe { host_log("Finished inject_botguard_functions script".to_string())? };
    Ok(())
}

fn descramble(scrambled: &str) -> Option<String> {
    let base64_mod = scrambled.replace('-', "+").replace('_', "/").replace('.', "=");
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64_mod).ok()?;
    let descrambled_bytes: Vec<u8> = bytes.into_iter().map(|b| b.wrapping_add(97)).collect();
    String::from_utf8(descrambled_bytes).ok()
}

fn parse_challenge_data(raw: &str) -> Option<(String, String, String)> {
    let scrambled: serde_json::Value = serde_json::from_str(raw).ok()?;
    let scrambled_arr = scrambled.as_array()?;
    
    let challenge_data = if scrambled_arr.len() > 1 && scrambled_arr[1].is_string() {
        let descrambled = descramble(scrambled_arr[1].as_str()?)?;
        let parsed: serde_json::Value = serde_json::from_str(&descrambled).ok()?;
        parsed.as_array()?.clone()
    } else {
        scrambled_arr.get(0)?.as_array()?.clone()
    };
    
    let interpreter = challenge_data.get(1)?
        .as_array()?
        .iter()
        .find(|v| v.is_string())?
        .as_str()?
        .to_string();
        
    let program = challenge_data.get(4)?.as_str()?.to_string();
    let global_name = challenge_data.get(5)?.as_str()?.to_string();
    
    Some((interpreter, program, global_name))
}

fn init_botguard() -> FnResult<bool> {
    if let Ok(Some(is_init)) = extism_pdk::var::get::<String>("bg_init") {
        if is_init == "true" {
            if let Ok(Some(expires_at_str)) = extism_pdk::var::get::<String>("bg_expires_at") {
                if let Ok(expires_at) = expires_at_str.parse::<u64>() {
                    let now = get_unix_timestamp();
                    if now < expires_at {
                        return Ok(false);
                    }
                }
            }
        }
    }
    
    inject_botguard_functions()?;
    
    let payload = format!(r#"["{}"]"#, REQUEST_KEY);
    let bg_res = do_botguard_http("https://www.youtube.com/api/jnn/v1/Create", &payload)?;
    
    if bg_res.status != 200 {
        return Err(extism_pdk::Error::msg(format!("JNN Create failed: {}", bg_res.status)).into());
    }
    
    let (interpreter, program, global_name) = parse_challenge_data(&bg_res.body)
        .ok_or_else(|| extism_pdk::Error::msg("Failed to parse or descramble JNN challenge data"))?;
    
    // Evaluate interpreter payload directly and force primitive return
    let safe_interpreter = format!("{}\n\"ok\";", interpreter);
    unsafe { host_log("Executing interpreter script".to_string())? };
    unsafe { host_execute_webview_js(safe_interpreter)? };
    unsafe { host_log("Finished interpreter script".to_string())? };
    
    let script1 = format!(
        r#"
        (async function() {{
            await window.loadBotGuard({{ globalName: "{}", program: "{}" }});
            var webPoSignalOutput = [];
            var botguardResponse = await window.snapshot(webPoSignalOutput);
            window.webPoSignalOutput = webPoSignalOutput;
            return botguardResponse;
        }})();
        "#,
        global_name, program
    );
    
    let botguard_response_json = unsafe {
        host_log("Executing script1 (snapshot)".to_string())?;
        let res = host_execute_webview_js(script1)?;
        host_log("Finished script1 (snapshot)".to_string())?;
        res
    };
    
    // The response is a JSON serialized string (e.g., "\"snapshot_data\""), so we parse it back to a raw string
    let parsed_bg_val: serde_json::Value = serde_json::from_str(&botguard_response_json).unwrap_or_default();
    let botguard_response = parsed_bg_val.as_str().unwrap_or("");
    unsafe { host_log(format!("BotGuard Snapshot response length: {}, starts with: {}", botguard_response.len(), botguard_response.chars().take(30).collect::<String>()))? };
    
    let it_payload = format!(r#"["{}", "{}"]"#, REQUEST_KEY, botguard_response);
    let it_res = do_botguard_http("https://www.youtube.com/api/jnn/v1/GenerateIT", &it_payload)?;
    unsafe { host_log(format!("GenerateIT HTTP Status: {}, Body: {}", it_res.status, it_res.body.chars().take(200).collect::<String>()))? };
    
    if it_res.status != 200 {
        return Err(extism_pdk::Error::msg(format!("JNN GenerateIT failed: {}", it_res.status)).into());
    }
    
    let it_json: serde_json::Value = serde_json::from_str(&it_res.body).unwrap_or_default();
    let integrity_token = it_json[0].as_str().unwrap_or("");
    let expires_in = it_json[1].as_i64().unwrap_or(0) as u64;
    
    // Convert base64 integrity token to Uint8Array string representation matching Metrolist base64ToByteString
    let mod_integrity = integrity_token.replace('-', "+").replace('_', "/").replace('.', "=");
    let decoded_it = general_purpose::STANDARD.decode(&mod_integrity)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(mod_integrity.trim_end_matches('=')))
        .unwrap_or_default();
    unsafe { host_log(format!("Decoded integrity_token u8 byte count: {}", decoded_it.len()))? };
    let it_u8_str = decoded_it.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",");
    
    let now = get_unix_timestamp();
    let expires_at = now + expires_in.saturating_sub(600); // 10 minute buffer
    extism_pdk::var::set("bg_expires_at", expires_at.to_string())?;
    
    let script2 = format!(
        r#"
        (async function() {{
            var getMinter = window.webPoSignalOutput[0];
            var integrityTokenU8 = new Uint8Array([{}]);
            var minterResult = getMinter(integrityTokenU8);
            if (minterResult && typeof minterResult.then === 'function') {{
                window.poTokenMinter = await minterResult;
            }} else {{
                window.poTokenMinter = minterResult;
            }}
            return "ok";
        }})();
        "#,
        it_u8_str
    );
    
    unsafe {
        host_log("Executing script2 (minter)".to_string())?;
        host_execute_webview_js(script2)?;
        host_log("Finished script2 (minter)".to_string())?;
    };
    
    extism_pdk::var::set("bg_init", "true")?;
    Ok(true)
}

fn generate_po_token(identifier: &str) -> FnResult<String> {
    let script = format!(
        r#"
        (async function() {{
            if (typeof window.poTokenMinter !== 'function') {{
                return "ERROR_MINTER_NOT_INITIALIZED";
            }}
            var identifierU8 = new Uint8Array([{}]);
            var mintResult = window.poTokenMinter(identifierU8);
            var result;
            if (mintResult && typeof mintResult.then === 'function') {{
                result = await mintResult;
            }} else {{
                result = mintResult;
            }}
            if (!result || typeof result.byteLength !== 'number') {{
                return "ERROR_MINTER_INVALID_RESULT";
            }}
            let binary = '';
            for (let i = 0; i < result.byteLength; i++) {{
                binary += String.fromCharCode(result[i]);
            }}
            console.log('[JS generate_po_token] byteLength: ' + result.byteLength);
            return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_');
        }})();
        "#,
        identifier.as_bytes().iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",")
    );
    let res_json = unsafe {
        host_log("Executing generate_po_token script".to_string())?;
        let result = host_execute_webview_js(script)?;
        host_log("Finished generate_po_token script".to_string())?;
        result
    };
    
    let parsed_val: serde_json::Value = serde_json::from_str(&res_json).unwrap_or_default();
    let token = parsed_val.as_str().unwrap_or("").to_string();
    if token.starts_with("ERROR_") || token.is_empty() {
        unsafe { host_log(format!("poToken generation failed in JS: {}. Resetting bg_init.", token))? };
        let _ = extism_pdk::var::set("bg_init", "false");
        return Err(extism_pdk::Error::msg(format!("poToken minter failed: {}", token)).into());
    }
    unsafe { host_log(format!("Generated full PoToken (len: {}): {}", token.len(), token))? };
    Ok(token)
}

fn find_visitor_data_in_json(val: &serde_json::Value) -> Option<String> {
    match val {
        serde_json::Value::String(s) => {
            // Metrolist: VISITOR_DATA_REGEX = Regex("^Cg[t|s]")
            if (s.starts_with("Cgt") || s.starts_with("Cgs")) && s.len() >= 22 {
                return Some(s.clone());
            }
            None
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(found) = find_visitor_data_in_json(item) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                if let Some(found) = find_visitor_data_in_json(v) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn get_visitor_data() -> FnResult<String> {
    let vd = get_storage("visitor_data").unwrap_or_default();
    if !vd.is_empty() {
        return Ok(vd);
    }
    
    // Metrolist: GET music.youtube.com/sw.js_data, strip ")]}'\n" XSSI prefix, search recursively for visitorData
    let res = do_http("GET", "https://music.youtube.com/sw.js_data", None, None)?;
    
    let body = if res.body.starts_with(")]}'") {
        res.body.chars().skip(5).collect::<String>()
    } else {
        res.body.clone()
    };
    
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    
    // Recursively search json[0][2] for a string matching Metrolist's VISITOR_DATA_REGEX "^Cg[ts]"
    if let Some(vd) = find_visitor_data_in_json(&json[0][2]) {
        unsafe { host_log(format!("Fetched visitorData from sw.js_data (first 20): {}", vd.chars().take(20).collect::<String>()))? };
        set_storage("visitor_data", &vd)?;
        return Ok(vd);
    }
    
    unsafe { host_log("WARNING: Could not extract visitorData from sw.js_data".to_string())? };
    Ok("".to_string())
}

// Client fallback chain, ordered like Metrolist's `defaultClients`: clients that need
// neither a poToken nor signatureTimestamp come first since they need no BotGuard/cipher
// work at all, and are tried before the much stricter WEB_REMIX.
#[derive(Clone)]
struct YtClient {
    name: &'static str,
    version: &'static str,
    client_id: &'static str,
    user_agent: &'static str,
    needs_potoken: bool,
    needs_sts: bool,
    include_ua_in_context: bool,
    is_embedded: bool,
    os_name: Option<&'static str>,
    os_version: Option<&'static str>,
    device_make: Option<&'static str>,
    device_model: Option<&'static str>,
    android_sdk_version: Option<&'static str>,
}

fn fallback_clients() -> Vec<YtClient> {
    vec![
        // 1. VISIONOS (Direct URL, lightweight)
        YtClient {
            name: "VISIONOS", version: "0.1", client_id: "101",
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
            needs_potoken: false, needs_sts: false, include_ua_in_context: false, is_embedded: false,
            os_name: Some("visionOS"), os_version: Some("1.3.21O771"),
            device_make: Some("Apple"), device_model: Some("RealityDevice14,1"),
            android_sdk_version: None,
        },
        // 2. ANDROID_VR 1.65.10 (Direct URL)
        YtClient {
            name: "ANDROID_VR", version: "1.65.10", client_id: "28",
            user_agent: "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
            needs_potoken: false, needs_sts: false, include_ua_in_context: true, is_embedded: false,
            os_name: Some("Android"), os_version: Some("12L"),
            device_make: Some("Oculus"), device_model: Some("Quest 3"),
            android_sdk_version: Some("32"),
        },
        // 3. ANDROID_VR 1.43.32 (Direct URL)
        YtClient {
            name: "ANDROID_VR", version: "1.43.32", client_id: "28",
            user_agent: "com.google.android.apps.youtube.vr.oculus/1.43.32 (Linux; U; Android 12; en_US; Quest 3; Build/SQ3A.220605.009.A1; Cronet/107.0.5284.2)",
            needs_potoken: false, needs_sts: false, include_ua_in_context: true, is_embedded: false,
            os_name: Some("Android"), os_version: Some("12"),
            device_make: Some("Oculus"), device_model: Some("Quest 3"),
            android_sdk_version: Some("32"),
        },
        // 4. IOS (Direct URL / iOS Native)
        YtClient {
            name: "IOS", version: "21.03.1", client_id: "5",
            user_agent: "com.google.ios.youtube/21.03.1 (iPhone16,2; U; CPU iOS 18_2 like Mac OS X;)",
            needs_potoken: false, needs_sts: false, include_ua_in_context: false, is_embedded: false,
            os_name: Some("iOS"), os_version: Some("18.2.22C152"),
            device_make: Some("Apple"), device_model: Some("iPhone16,2"),
            android_sdk_version: None,
        },
        // 5. WEB_REMIX (Main high-quality YT Music client: 256kbps Opus/AAC with cipher + n-transform)
        YtClient {
            name: "WEB_REMIX", version: "1.20260114.03.00", client_id: "67",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
            needs_potoken: true, needs_sts: true, include_ua_in_context: false, is_embedded: false,
            os_name: None, os_version: None, device_make: None, device_model: None,
            android_sdk_version: None,
        },
        // 6. TVHTML5 (Smart TV client)
        YtClient {
            name: "TVHTML5", version: "7.20260114.12.00", client_id: "7",
            user_agent: "Mozilla/5.0 (ChromiumStylePlatform) Cobalt/25.lts.30.1034943-gold (unlike Gecko), Unknown_TV_Unknown_0/Unknown (Unknown, Unknown)",
            needs_potoken: true, needs_sts: true, include_ua_in_context: true, is_embedded: false,
            os_name: None, os_version: None, device_make: None, device_model: None,
            android_sdk_version: None,
        },
        // 7. TVHTML5_SIMPLY_EMBEDDED_PLAYER (Age-restriction & login bypass via Reddit embed)
        YtClient {
            name: "TVHTML5_SIMPLY_EMBEDDED_PLAYER", version: "2.0", client_id: "85",
            user_agent: "Mozilla/5.0 (PlayStation; PlayStation 4/12.02) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/15.4 Safari/605.1.15",
            needs_potoken: false, needs_sts: true, include_ua_in_context: false, is_embedded: true,
            os_name: None, os_version: None, device_make: None, device_model: None,
            android_sdk_version: None,
        },
        // 8. WEB_CREATOR (Creator/studio fallback)
        YtClient {
            name: "WEB_CREATOR", version: "1.20260114.05.00", client_id: "62",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
            needs_potoken: true, needs_sts: true, include_ua_in_context: false, is_embedded: false,
            os_name: None, os_version: None, device_make: None, device_model: None,
            android_sdk_version: None,
        },
    ]
}

fn build_player_body(client: &YtClient, track_id: &str, vd: &str, po_token: Option<&str>, sts: Option<i64>) -> serde_json::Value {
    let mut client_obj = serde_json::json!({
        "clientName": client.name,
        "clientVersion": client.version,
        "gl": "US",
        "hl": "en",
        "visitorData": vd,
    });
    if client.include_ua_in_context {
        client_obj["userAgent"] = serde_json::json!(client.user_agent);
    }
    if let Some(v) = client.os_name { client_obj["osName"] = serde_json::json!(v); }
    if let Some(v) = client.os_version { client_obj["osVersion"] = serde_json::json!(v); }
    if let Some(v) = client.device_make { client_obj["deviceMake"] = serde_json::json!(v); }
    if let Some(v) = client.device_model { client_obj["deviceModel"] = serde_json::json!(v); }
    if let Some(v) = client.android_sdk_version { client_obj["androidSdkVersion"] = serde_json::json!(v); }

    let mut context_obj = serde_json::json!({
        "client": client_obj,
        "request": { "internalExperimentFlags": [], "useSsl": true },
        "user": { "lockedSafetyMode": false }
    });

    if client.is_embedded {
        context_obj["thirdParty"] = serde_json::json!({
            "embedUrl": "https://www.reddit.com/"
        });
    }

    let mut body = serde_json::json!({
        "context": context_obj,
        "videoId": track_id,
        "contentCheckOk": true,
        "racyCheckOk": true,
    });

    if client.needs_sts {
        if let Some(sts_val) = sts {
            body["playbackContext"] = serde_json::json!({
                "contentPlaybackContext": { "signatureTimestamp": sts_val }
            });
        }
    }

    if client.needs_potoken {
        if let Some(pt) = po_token {
            body["serviceIntegrityDimensions"] = serde_json::json!({ "poToken": pt });
        }
    }

    body
}

fn request_player(client: &YtClient, body: &serde_json::Value, vd: &str) -> FnResult<serde_json::Value> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());
    headers.insert("Accept-Language".to_string(), "en-US,en;q=0.9".to_string());
    headers.insert("Cache-Control".to_string(), "no-cache".to_string());
    headers.insert("User-Agent".to_string(), client.user_agent.to_string());
    headers.insert("X-Goog-Api-Format-Version".to_string(), "1".to_string());
    headers.insert("X-YouTube-Client-Name".to_string(), client.client_id.to_string());
    headers.insert("X-YouTube-Client-Version".to_string(), client.version.to_string());
    headers.insert("X-Origin".to_string(), "https://music.youtube.com".to_string());
    headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());
    headers.insert("X-Goog-Visitor-Id".to_string(), vd.to_string());

    let res = do_http(
        "POST",
        "https://music.youtube.com/youtubei/v1/player?prettyPrint=false",
        Some(headers),
        Some(body.to_string()),
    )?;
    Ok(serde_json::from_str(&res.body).unwrap_or_default())
}

// poTokens are base64url (`-`/`_`) but may retain `=` padding, which is not a valid
// query-string character — percent-encode before appending as `pot=`.
fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}


fn browse_innertube(browse_id: &str, params: Option<&str>, gl: Option<&str>) -> FnResult<serde_json::Value> {
    let vd = get_visitor_data().unwrap_or_default();
    let country = gl.unwrap_or("US");
    let mut body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20260114.03.00",
                "gl": country,
                "hl": "en",
                "visitorData": vd
            }
        },
        "browseId": browse_id
    });

    if let Some(p) = params {
        body["params"] = serde_json::json!(p);
    }

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0".to_string());
    headers.insert("X-YouTube-Client-Name".to_string(), "67".to_string());
    headers.insert("X-YouTube-Client-Version".to_string(), "1.20260114.03.00".to_string());
    headers.insert("X-Goog-Visitor-Id".to_string(), vd);

    let res = do_http("POST", "https://music.youtube.com/youtubei/v1/browse?prettyPrint=false", Some(headers), Some(body.to_string()))?;
    Ok(serde_json::from_str(&res.body).unwrap_or_default())
}


fn parse_playlist_shelf_tracks(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut items = Vec::new();
    let tabs = json["contents"]["twoColumnBrowseResultsRenderer"]["secondaryContents"]["sectionListRenderer"]["contents"].as_array()
        .or_else(|| json["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array());

    if let Some(sections) = tabs {
        for sec in sections {
            let shelf = sec.get("musicPlaylistShelfRenderer")
                .or_else(|| sec.get("musicShelfRenderer"));
            if let Some(s) = shelf {
                if let Some(item_list) = s["contents"].as_array() {
                    let mut rank = 1;
                    for it in item_list {
                        if let Some(r) = it.get("musicResponsiveListItemRenderer") {
                            let vid = r["playlistItemData"]["videoId"].as_str()
                                .or_else(|| r["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .or_else(|| r["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .unwrap_or("").to_string();
                            if vid.is_empty() { continue; }

                            let raw_title = r["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let raw_artist = r["flexColumns"].get(1)
                                .and_then(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .unwrap_or("").to_string();
                            let (title, artist) = clean_title_and_artist(&raw_title, &raw_artist);
                            let cover = r["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"].as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|t| t["url"].as_str())
                                .map(|s| s.to_string());

                            let dur_ms = r.get("fixedColumns")
                                .and_then(|fc| fc.get(0))
                                .and_then(|c| c["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .and_then(parse_duration_ms);

                            items.push(ModuleItem::Track(TrackResult {
                                id: vid,
                                title,
                                artist,
                                album: None,
                                cover_art_url: cover,
                                stream_url: None,
                                quality_hint: Some(format!("#{}", rank)),
                                duration_ms: dur_ms,
                                plays: None,
                            }));
                            rank += 1;
                        }
                    }
                    if !items.is_empty() { return items; }
                }
            }
        }
    }
    items
}

fn fetch_chart_carousel_playlist(gl: Option<&str>, playlist_index: usize, preferred_title: Option<&str>) -> Vec<ModuleItem> {
    if let Ok(json) = browse_innertube("FEmusic_charts", None, gl) {
        if let Some(tabs) = json["contents"]["singleColumnBrowseResultsRenderer"]["tabs"].as_array() {
            if let Some(sections) = tabs.get(0).and_then(|t| t["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array()) {
                // If a preferred chart title is specified (e.g. "Hindi", "Top 100", etc.), look for that shelf item first
                if let Some(pref) = preferred_title {
                    let pref_lower = pref.to_lowercase();
                    for sec in sections {
                        if let Some(carousel) = sec.get("musicCarouselShelfRenderer") {
                            if let Some(playlists) = carousel["contents"].as_array() {
                                for p in playlists {
                                    let item_title = p["musicTwoRowItemRenderer"]["title"]["runs"][0]["text"].as_str().unwrap_or("").to_lowercase();
                                    if item_title.contains(&pref_lower) {
                                        if let Some(browse_id) = p["musicTwoRowItemRenderer"]["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str() {
                                            if let Ok(pl_json) = browse_innertube(browse_id, None, gl) {
                                                let parsed = parse_playlist_shelf_tracks(&pl_json);
                                                if !parsed.is_empty() {
                                                    return parsed;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Default carousel index fallback
                for sec in sections {
                    if let Some(carousel) = sec.get("musicCarouselShelfRenderer") {
                        if let Some(playlists) = carousel["contents"].as_array() {
                            if !playlists.is_empty() {
                                let idx = playlist_index.min(playlists.len() - 1);
                                if let Some(browse_id) = playlists[idx]["musicTwoRowItemRenderer"]["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str() {
                                    if let Ok(pl_json) = browse_innertube(browse_id, None, gl) {
                                        let parsed = parse_playlist_shelf_tracks(&pl_json);
                                        if !parsed.is_empty() {
                                            return parsed;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Vec::new()
}

fn parse_charts_tracks(json: &serde_json::Value, target_shelf: Option<&str>) -> Vec<ModuleItem> {
    let mut items = Vec::new();
    let sections = json["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array();
    
    if let Some(sections_arr) = sections {
        for section in sections_arr {
            // 1. Check musicCarouselShelfRenderer (Modern InnerTube Charts Structure as in Metrolist)
            if let Some(carousel) = section.get("musicCarouselShelfRenderer") {
                let title = carousel["header"]["musicCarouselShelfBasicHeaderRenderer"]["title"]["runs"][0]["text"].as_str()
                    .unwrap_or("").to_lowercase();
                
                if let Some(target) = target_shelf {
                    if (target == "trending" || target == "viral") && !title.contains("trending") && !title.contains("viral") {
                        continue;
                    }
                    if target == "top_songs" && !title.contains("top songs") && !title.contains("top") && !title.contains("chart") && !title.is_empty() {
                        // Skip if it's explicitly artists or videos and we want songs
                        if title.contains("artist") || title.contains("video") {
                            continue;
                        }
                    }
                }

                if let Some(item_list) = carousel["contents"].as_array() {
                    let mut rank = 1;
                    for item in item_list {
                        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                            let id = renderer["playlistItemData"]["videoId"].as_str()
                                .or_else(|| renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .or_else(|| renderer["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .unwrap_or("").to_string();
                            if id.is_empty() { continue; }

                            let raw_title = renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let raw_artist = renderer["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let (title_str, artist_str) = clean_title_and_artist(&raw_title, &raw_artist);
                            let cover = renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"].as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|t| t["url"].as_str())
                                .map(|s| s.to_string());

                            let rank_str = renderer["flexColumns"].get(2)
                                .and_then(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| format!("#{}", rank));

                            let dur_ms = renderer.get("fixedColumns")
                                .and_then(|fc| fc.get(0))
                                .and_then(|c| c["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .and_then(parse_duration_ms);

                            items.push(ModuleItem::Track(TrackResult {
                                id,
                                title: title_str,
                                artist: artist_str,
                                album: None,
                                cover_art_url: cover,
                                stream_url: None,
                                quality_hint: Some(rank_str),
                                duration_ms: dur_ms,
                                plays: None,
                            }));
                            rank += 1;
                        }
                    }
                    if !items.is_empty() { return items; }
                }
            }

            // 2. Fallback to classic musicShelfRenderer
            let shelf = section.get("musicShelfRenderer")
                .or_else(|| section["itemSectionRenderer"]["contents"][0].get("musicShelfRenderer"));
            if let Some(shelf_renderer) = shelf {
                if let Some(item_list) = shelf_renderer["contents"].as_array() {
                    let mut rank = 1;
                    for item in item_list {
                        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                            let id = renderer["playlistItemData"]["videoId"].as_str()
                                .or_else(|| renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .or_else(|| renderer["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .unwrap_or("").to_string();
                            if id.is_empty() { continue; }

                            let raw_title = renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let raw_artist = renderer["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let (title_str, artist_str) = clean_title_and_artist(&raw_title, &raw_artist);
                            let cover = renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"].as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|t| t["url"].as_str())
                                .map(|s| s.to_string());

                            let dur_ms = renderer.get("fixedColumns")
                                .and_then(|fc| fc.get(0))
                                .and_then(|c| c["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .and_then(parse_duration_ms);

                            items.push(ModuleItem::Track(TrackResult {
                                id,
                                title: title_str,
                                artist: artist_str,
                                album: None,
                                cover_art_url: cover,
                                stream_url: None,
                                quality_hint: Some(format!("#{}", rank)),
                                duration_ms: dur_ms,
                                plays: None,
                            }));
                            rank += 1;
                        }
                    }
                    if !items.is_empty() { return items; }
                }
            }
        }
    }
    items
}

fn parse_new_releases_albums(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut albums = Vec::new();
    let mut singles = Vec::new();

    let sections = json.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .or_else(|| json.pointer("/contents/sectionListRenderer/contents"))
        .and_then(|v| v.as_array());

    if let Some(sections_arr) = sections {
        for section in sections_arr {
            // 1. Identify the exact New Releases shelf (like Metrolist)
            let more_endpoint = section.pointer("/musicCarouselShelfRenderer/header/musicCarouselShelfBasicHeaderRenderer/moreContentButton/buttonRenderer/navigationEndpoint/browseEndpoint/browseId")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let shelf_title = section.pointer("/musicCarouselShelfRenderer/header/musicCarouselShelfBasicHeaderRenderer/title/runs/0/text")
                .or_else(|| section.pointer("/musicShelfRenderer/title/runs/0/text"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_lowercase();

            let is_new_releases_shelf = more_endpoint == "FEmusic_new_releases_albums"
                || shelf_title.contains("new albums")
                || shelf_title.contains("new releases")
                || section.get("gridRenderer").is_some();

            if !is_new_releases_shelf && !shelf_title.is_empty() {
                continue;
            }

            let contents = section.pointer("/musicCarouselShelfRenderer/contents")
                .or_else(|| section.pointer("/gridRenderer/items"))
                .or_else(|| section.pointer("/musicShelfRenderer/contents"))
                .and_then(|v| v.as_array());

            if let Some(items) = contents {
                for item in items {
                    if let Some(renderer) = item.get("musicTwoRowItemRenderer") {
                        let browse_id = renderer.pointer("/navigationEndpoint/browseEndpoint/browseId")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        if browse_id.is_empty() {
                            continue;
                        }

                        let title = renderer.pointer("/title/runs/0/text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        if title.is_empty() {
                            continue;
                        }

                        // Parse subtitle runs split by separator " • " (Metrolist approach)
                        let mut release_type = String::new();
                        let mut artist_names = Vec::new();
                        let mut has_verified_artist_channel = false;
                        let mut year = None;

                        if let Some(runs) = renderer.pointer("/subtitle/runs").and_then(|v| v.as_array()) {
                            let mut segments: Vec<Vec<&serde_json::Value>> = Vec::new();
                            let mut current_segment = Vec::new();

                            for r in runs {
                                let text = r.get("text").and_then(|v| v.as_str()).unwrap_or_default().trim();
                                if text == "•" {
                                    if !current_segment.is_empty() {
                                        segments.push(current_segment);
                                        current_segment = Vec::new();
                                    }
                                } else if !text.is_empty() {
                                    current_segment.push(r);
                                }
                            }
                            if !current_segment.is_empty() {
                                segments.push(current_segment);
                            }

                            if !segments.is_empty() {
                                let first_seg_text = segments[0].iter()
                                    .filter_map(|r| r.get("text").and_then(|v| v.as_str()))
                                    .collect::<Vec<_>>()
                                    .join("");
                                release_type = first_seg_text.to_lowercase();

                                let artist_seg_idx = if segments.len() >= 2 && (release_type.contains("album") || release_type.contains("single") || release_type.contains("ep")) {
                                    1
                                } else {
                                    0
                                };

                                if artist_seg_idx < segments.len() {
                                    for r in &segments[artist_seg_idx] {
                                        if let Some(t) = r.get("text").and_then(|v| v.as_str()) {
                                            let trimmed = t.trim();
                                            if trimmed.len() == 4 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                                                year = Some(trimmed.to_string());
                                                continue;
                                            }
                                            if trimmed != "," && trimmed != "&" && !trimmed.is_empty() {
                                                artist_names.push(trimmed);
                                            }
                                            if let Some(artist_id) = r.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()) {
                                                if artist_id.starts_with("UC") {
                                                    has_verified_artist_channel = true;
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(last_seg) = segments.last() {
                                    for r in last_seg {
                                        if let Some(t) = r.get("text").and_then(|v| v.as_str()) {
                                            let trimmed = t.trim();
                                            if trimmed.len() == 4 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                                                year = Some(trimmed.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        let artist = if !artist_names.is_empty() {
                            artist_names.join(", ")
                        } else {
                            "Unknown Artist".to_string()
                        };

                        let raw_cover = renderer.pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.last())
                            .and_then(|t| t.get("url"))
                            .and_then(|u| u.as_str());
                        let cover = upscale_yt_art(raw_cover);

                        let album_item = ModuleItem::Album(AlbumItem {
                            id: browse_id,
                            title,
                            artist,
                            year,
                            cover_art_url: cover,
                        });

                        let is_full_album = release_type.contains("album") || release_type.contains("ep");

                        if is_full_album {
                            albums.push((has_verified_artist_channel, album_item));
                        } else {
                            singles.push((has_verified_artist_channel, album_item));
                        }
                    }
                }
            }
        }
    }

    albums.sort_by(|a, b| b.0.cmp(&a.0));
    singles.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result: Vec<ModuleItem> = albums.into_iter().map(|(_, item)| item).collect();
    if result.len() < 4 {
        result.extend(singles.into_iter().map(|(_, item)| item));
    }
    result
}

fn default_new_releases() -> Vec<ModuleItem> {
    vec![
        ModuleItem::Album(AlbumItem {
            id: "MPREb_kavinsky_reborn".to_string(),
            title: "Reborn".to_string(),
            artist: "Kavinsky".to_string(),
            year: Some("2024".to_string()),
            cover_art_url: Some("https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?q=80&w=800&auto=format&fit=crop".to_string()),
        }),
        ModuleItem::Album(AlbumItem {
            id: "MPREb_weeknd_hurry".to_string(),
            title: "Hurry Up Tomorrow".to_string(),
            artist: "The Weeknd".to_string(),
            year: Some("2025".to_string()),
            cover_art_url: Some("https://images.unsplash.com/photo-1514525253161-7a46d19cd819?q=80&w=800&auto=format&fit=crop".to_string()),
        }),
        ModuleItem::Album(AlbumItem {
            id: "MPREb_justice_hyperdrama".to_string(),
            title: "Hyperdrama".to_string(),
            artist: "Justice".to_string(),
            year: Some("2024".to_string()),
            cover_art_url: Some("https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=800&auto=format&fit=crop".to_string()),
        }),
        ModuleItem::Album(AlbumItem {
            id: "MPREb_charli_brat".to_string(),
            title: "BRAT".to_string(),
            artist: "Charli xcx".to_string(),
            year: Some("2024".to_string()),
            cover_art_url: Some("https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?q=80&w=800&auto=format&fit=crop".to_string()),
        }),
    ]
}




fn parse_editorial_spotlights(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut spotlights = Vec::new();
    let sections = match json.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents").and_then(|v| v.as_array()) {
        Some(s) => s,
        None => return spotlights,
    };

    for sec in sections {
        if let Some(items) = sec.pointer("/musicCarouselShelfRenderer/contents").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(r) = item.get("musicTwoRowItemRenderer") {
                    let title = r.pointer("/title/runs/0/text").and_then(|v| v.as_str()).unwrap_or_default();
                    let artist = r.pointer("/subtitle/runs/0/text").and_then(|v| v.as_str()).unwrap_or_default();
                    let browse_id = r.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()).unwrap_or_default();
                    let cover_art_url = r.pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|t| t.get("url"))
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string());

                    if !title.is_empty() && !browse_id.is_empty() {
                        spotlights.push(ModuleItem::Spotlight(EditorialSpotlight {
                            id: browse_id.to_string(),
                            title: title.to_string(),
                            artist: artist.to_string(),
                            cover_art_url,
                            description: Some("Featured Release • Lossless Master Edition".to_string()),
                            release_year: Some("2026".to_string()),
                            track_count: Some(10),
                        }));

                        if spotlights.len() >= 24 {
                            return spotlights;
                        }
                    }
                }
            }
        }
    }
    spotlights
}

fn curated_categories() -> Vec<ModuleItem> {
    vec![
        ModuleItem::Genre(GenreItem { id: "electronic".into(), title: "Electronic".into(), endpoint_params: None, color_hex: Some("#19A7CE".into()) }),
        ModuleItem::Genre(GenreItem { id: "classical".into(), title: "Classical".into(), endpoint_params: None, color_hex: Some("#9B59B6".into()) }),
        ModuleItem::Genre(GenreItem { id: "jazz".into(), title: "Jazz & Soul".into(), endpoint_params: None, color_hex: Some("#D35400".into()) }),
        ModuleItem::Genre(GenreItem { id: "ambient".into(), title: "Ambient".into(), endpoint_params: None, color_hex: Some("#16A085".into()) }),
        ModuleItem::Genre(GenreItem { id: "hiphop".into(), title: "Hip-Hop".into(), endpoint_params: None, color_hex: Some("#C0392B".into()) }),
        ModuleItem::Genre(GenreItem { id: "rock".into(), title: "Rock".into(), endpoint_params: None, color_hex: Some("#2C3E50".into()) }),
        ModuleItem::Genre(GenreItem { id: "focus".into(), title: "Focus / Study".into(), endpoint_params: None, color_hex: Some("#2980B9".into()) }),
        ModuleItem::Genre(GenreItem { id: "global_top".into(), title: "Global Top".into(), endpoint_params: None, color_hex: Some("#B58E62".into()) }),
    ]
}

fn parse_trending_albums(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut official_albums = Vec::new();
    let mut other_albums = Vec::new();
    let sections = json.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .or_else(|| json.pointer("/contents/sectionListRenderer/contents"))
        .and_then(|v| v.as_array());

    if let Some(secs) = sections {
        for sec in secs {
            let shelf_contents = sec.pointer("/musicCarouselShelfRenderer/contents")
                .or_else(|| sec.pointer("/itemSectionRenderer/contents/0/musicShelfRenderer/contents"))
                .or_else(|| sec.pointer("/gridRenderer/items"))
                .and_then(|v| v.as_array());

            if let Some(contents) = shelf_contents {
                for item in contents {
                    let renderer = item.get("musicTwoRowItemRenderer")
                        .or_else(|| item.get("musicResponsiveListItemRenderer"));

                    if let Some(r) = renderer {
                        let browse_id = r.pointer("/navigationEndpoint/browseEndpoint/browseId")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        let page_type = r.pointer("/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        let is_album = browse_id.starts_with("MPREb_") || page_type == "MUSIC_PAGE_TYPE_ALBUM";
                        if !is_album {
                            continue;
                        }

                        let playlist_id = r.pointer("/thumbnailOverlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        let is_official_package = playlist_id.starts_with("OLAK5uy_");

                        let title = r.pointer("/title/runs/0/text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();

                        if title.is_empty() || is_junk_title(&title) {
                            continue;
                        }

                        let subtitle_runs = r.pointer("/subtitle/runs").and_then(|v| v.as_array());
                        let mut artist_names = Vec::new();
                        let mut has_verified_artist = false;
                        let mut year = None;
                        let mut release_type = String::new();

                        if let Some(runs) = subtitle_runs {
                            let mut segments: Vec<Vec<&serde_json::Value>> = Vec::new();
                            let mut current = Vec::new();
                            for run in runs {
                                let t = run.get("text").and_then(|v| v.as_str()).unwrap_or("");
                                if t == " • " || t == "•" {
                                    if !current.is_empty() { segments.push(current); }
                                    current = Vec::new();
                                } else {
                                    current.push(run);
                                }
                            }
                            if !current.is_empty() { segments.push(current); }

                            if let Some(first_seg) = segments.first() {
                                if let Some(t) = first_seg.first().and_then(|r| r.get("text")).and_then(|v| v.as_str()) {
                                    release_type = t.to_lowercase();
                                }
                            }

                            for seg in segments.iter().skip(1) {
                                for r in seg {
                                    if let Some(name) = r.get("text").and_then(|v| v.as_str()) {
                                        let trimmed = name.trim();
                                        if trimmed.len() == 4 && trimmed.chars().all(|c| c.is_ascii_digit()) {
                                            year = Some(trimmed.to_string());
                                        } else if !trimmed.is_empty() && trimmed != "&" && trimmed != "," {
                                            if let Some(c_id) = r.pointer("/navigationEndpoint/browseEndpoint/browseId").and_then(|v| v.as_str()) {
                                                if c_id.starts_with("UC") {
                                                    has_verified_artist = true;
                                                }
                                            }
                                            artist_names.push(trimmed.to_string());
                                        }
                                    }
                                }
                            }
                        }

                        let is_studio_album = release_type.contains("album") || release_type.contains("ep") || release_type.is_empty();
                        if !is_studio_album {
                            continue;
                        }

                        let artist = if !artist_names.is_empty() {
                            artist_names.join(", ")
                        } else {
                            "Unknown Artist".to_string()
                        };

                        if artist.eq_ignore_ascii_case("unknown artist") || is_junk_title(&artist) {
                            continue;
                        }

                        let cover_art_url = r.pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.last())
                            .and_then(|t| t.get("url"))
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string());

                        let item = ModuleItem::Album(AlbumItem {
                            id: browse_id,
                            title,
                            artist,
                            year,
                            cover_art_url,
                        });

                        if is_official_package || has_verified_artist {
                            official_albums.push(item);
                        } else {
                            other_albums.push(item);
                        }
                    }
                }
            }
        }
    }

    if !official_albums.is_empty() {
        official_albums
    } else {
        other_albums
    }
}

fn parse_trending_artists(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut items = Vec::new();
    let sections = json.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .and_then(|v| v.as_array());

    if let Some(secs) = sections {
        for sec in secs {
            let shelf_items = sec.pointer("/musicCarouselShelfRenderer/contents")
                .or_else(|| sec.pointer("/itemSectionRenderer/contents/0/musicShelfRenderer/contents"))
                .and_then(|v| v.as_array());

            if let Some(s_items) = shelf_items {
                for item in s_items {
                    let renderer = item.get("musicTwoRowItemRenderer")
                        .or_else(|| item.get("musicResponsiveListItemRenderer"));

                    if let Some(r) = renderer {
                        let browse_id = r.pointer("/navigationEndpoint/browseEndpoint/browseId")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        let name = r.pointer("/title/runs/0/text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        let subscribers = r.pointer("/subtitle/runs/0/text")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        let avatar_url = r.pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.last())
                            .and_then(|t| t.get("url"))
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string());

                        if browse_id.starts_with("UC") || browse_id.starts_with("FEmusic_artist") {
                            items.push(ModuleItem::Artist(ArtistItem {
                                id: browse_id.to_string(),
                                name: name.to_string(),
                                avatar_url,
                                subscribers,
                            }));
                        }
                    }
                }
            }
        }
    }
    items
}

fn parse_moods_and_genres(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut items = Vec::new();
    let sections = json.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .or_else(|| json.pointer("/contents/sectionListRenderer/contents"))
        .and_then(|v| v.as_array());
    
    if let Some(sections_arr) = sections {
        for section in sections_arr {
            let grid_items = section.pointer("/gridRenderer/items")
                .or_else(|| section.pointer("/musicCarouselShelfRenderer/contents"))
                .or_else(|| section.pointer("/itemSectionRenderer/contents/0/gridRenderer/items"))
                .and_then(|v| v.as_array());

            if let Some(grid) = grid_items {
                for item in grid {
                    if let Some(btn) = item.get("musicNavigationButtonRenderer") {
                        let title = btn.pointer("/buttonText/runs/0/text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        if title.is_empty() { continue; }

                        let params = btn.pointer("/clickCommand/browseEndpoint/params")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let raw_browse_id = btn.pointer("/clickCommand/browseEndpoint/browseId")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&title);
                        let browse_id = match &params {
                            Some(p) => format!("{}:{}", raw_browse_id, p),
                            None => format!("{}:{}", raw_browse_id, title),
                        };

                        let color_hex = btn.pointer("/solid/leftStripeColor")
                            .and_then(|v| v.as_u64())
                            .map(|c| format!("#{:06X}", c & 0xFFFFFF));

                        items.push(ModuleItem::Genre(GenreItem {
                            id: browse_id,
                            title,
                            endpoint_params: params,
                            color_hex,
                        }));
                    }
                }
            }
        }
    }
    items
}

fn do_search(input: String) -> FnResult<String> {
    let query: String = serde_json::from_str(&input)?;
    unsafe { host_log(format!("WASM searching for: {}", query))? };
    
    let vd = get_visitor_data().unwrap_or_default();
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.01.00",
                "visitorData": vd
            }
        },
        "query": query,
        "params": "Eg-KAQwIABAAGAAgACgAMABqChAEEAMQCRAFEAo%3D"
    });
    
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());
    
    let res = do_http("POST", "https://music.youtube.com/youtubei/v1/search", Some(headers), Some(body.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&res.body).unwrap_or_default();
    
    let mut tracks = Vec::new();
    let single_col = json["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array();
    let direct_sec = json["contents"]["sectionListRenderer"]["contents"].as_array();
    let contents = single_col.or(direct_sec);

    if let Some(sections) = contents {
        for section in sections {
            let shelf_contents = section["musicShelfRenderer"]["contents"].as_array()
                .or_else(|| section["musicCardShelfRenderer"]["contents"].as_array());

            if let Some(items) = shelf_contents {
                for item in items {
                    if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                        if let Some(track) = parse_search_track_renderer(renderer) {
                            tracks.push(track);
                        }
                    }
                }
            }
        }
    }
    
    Ok(serde_json::to_string(&tracks)?)
}


fn parse_duration_ms(text: &str) -> Option<u64> {
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() == 2 {
        let mins = parts[0].trim().parse::<u64>().ok()?;
        let secs = parts[1].trim().parse::<u64>().ok()?;
        Some((mins * 60 + secs) * 1000)
    } else if parts.len() == 3 {
        let hrs = parts[0].trim().parse::<u64>().ok()?;
        let mins = parts[1].trim().parse::<u64>().ok()?;
        let secs = parts[2].trim().parse::<u64>().ok()?;
        Some((hrs * 3600 + mins * 60 + secs) * 1000)
    } else {
        None
    }
}

fn upscale_yt_art(url: Option<&str>) -> Option<String> {
    url.map(|u| {
        if u.contains("=w") || u.contains("=s") {
            let base = u.split('=').next().unwrap_or(u);
            format!("{}=w544-h544-l90-rj", base)
        } else {
            u.to_string()
        }
    })
}

fn do_search_categorized(input: String) -> FnResult<String> {
    let req: SearchQueryInput = serde_json::from_str(&input)?;
    unsafe { host_log(format!("WASM do_search_categorized for: {} (filter: {:?})", req.query, req.filter))? };
    
    let vd = get_visitor_data().unwrap_or_default();
    
    let filter_param = match req.filter.as_deref() {
        Some("songs") => Some("EgWKAQIIAWoKEAkQBRAKEAMQBA%3D%3D"),
        Some("albums") => Some("EgWKAQIYAWoKEAkQChAFEAMQBA%3D%3D"),
        Some("artists") => Some("EgWKAQIgAWoKEAkQChAFEAMQBA%3D%3D"),
        Some("playlists") => Some("EgeKAQQoADgBagwQDhAKEAMQBRAJEAQ%3D"),
        _ => None,
    };

    let mut body_json = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20250101.01.00",
                "visitorData": vd
            }
        },
        "query": req.query
    });

    if let Some(param) = filter_param {
        body_json["params"] = serde_json::Value::String(param.to_string());
    }

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());
    headers.insert("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string());

    let res = do_http("POST", "https://music.youtube.com/youtubei/v1/search", Some(headers), Some(body_json.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&res.body).unwrap_or_default();

    let mut sections: Vec<SearchCategorySection> = Vec::new();
    let mut continuation_token: Option<String> = None;

    let mut item_section_songs: Vec<SearchItem> = Vec::new();
    let mut item_section_albums: Vec<SearchItem> = Vec::new();
    let mut item_section_artists: Vec<SearchItem> = Vec::new();
    let mut item_section_playlists: Vec<SearchItem> = Vec::new();
    let mut item_section_videos: Vec<SearchItem> = Vec::new();

    if let Some(contents) = json["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array() {
        for section in contents {
            // 1. Top Result Hero Card
            if let Some(card) = section.get("musicCardShelfRenderer") {
                let title = card["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                let subtitle_runs = card["subtitle"]["runs"].as_array();
                let subtitle = subtitle_runs.map(|runs| {
                    runs.iter().filter_map(|r| r["text"].as_str()).collect::<Vec<_>>().join("")
                }).unwrap_or_default();
                
                let browse_id = card["onTap"]["browseEndpoint"]["browseId"].as_str().unwrap_or("");
                let video_id = card["onTap"]["watchEndpoint"]["videoId"].as_str().unwrap_or("");
                let sub_lower = subtitle.to_lowercase();
                
                // Reject podcasts, episodes, user profiles, or stations from being the Top Result
                let is_non_music = sub_lower.starts_with("episode")
                    || sub_lower.starts_with("podcast")
                    || sub_lower.starts_with("profile")
                    || sub_lower.starts_with("station")
                    || sub_lower.starts_with("radio")
                    || browse_id.starts_with("MPED")
                    || browse_id.starts_with("MPSP")
                    || browse_id.starts_with("VLRD");

                if !is_non_music && (!browse_id.is_empty() || !video_id.is_empty()) {
                    let raw_cover = card["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                        .as_array()
                        .and_then(|arr| arr.last())
                        .and_then(|t| t["url"].as_str())
                        .or_else(|| {
                            card["header"]["musicCardShelfHeaderBasicRenderer"]["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                                .as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|t| t["url"].as_str())
                        })
                        .or_else(|| card["thumbnail"]["thumbnails"][0]["url"].as_str());
                    let cover = upscale_yt_art(raw_cover);
                    
                    let item_type = if browse_id.starts_with("UC") || browse_id.contains("artist") {
                        "artist".to_string()
                    } else if browse_id.starts_with("MPREb") || browse_id.contains("album") {
                        "album".to_string()
                    } else if browse_id.starts_with("VL") {
                        "playlist".to_string()
                    } else {
                        "song".to_string()
                    };

                    let card_id = if !browse_id.is_empty() { browse_id.to_string() } else { video_id.to_string() };

                    let top_result = TopResultItem {
                        id: card_id,
                        title,
                        subtitle,
                        item_type,
                        cover_art_url: cover,
                        provider_id: Some("youtube-wasm".to_string()),
                        provider_name: Some("YouTube Music".to_string()),
                    };

                    let mut top_items = vec![SearchItem::TopResult(top_result)];

                    // Check card contents (sub-tracks)
                    if let Some(sub_items) = card["contents"].as_array() {
                        for sub in sub_items {
                            if let Some(renderer) = sub.get("musicResponsiveListItemRenderer") {
                                if let Some(track) = parse_search_track_renderer(renderer) {
                                    top_items.push(SearchItem::Track(track));
                                }
                            }
                        }
                    }

                    sections.push(SearchCategorySection {
                        category: "Top Result".to_string(),
                        items: top_items,
                    });
                }
            }
            // 2. Grouped Shelf (e.g. Filtered Search Shelves)
            else if let Some(shelf) = section.get("musicShelfRenderer") {
                let raw_title = shelf["title"]["runs"][0]["text"].as_str()
                    .or_else(|| shelf["header"]["musicShelfHeaderBasicRenderer"]["title"]["runs"][0]["text"].as_str())
                    .unwrap_or("").to_string();
                
                let category_name = if raw_title.contains("Song") || raw_title.contains("Track") {
                    "Songs"
                } else if raw_title.contains("Album") {
                    "Albums"
                } else if raw_title.contains("Artist") {
                    "Artists"
                } else if raw_title.contains("Playlist") {
                    "Playlists"
                } else if raw_title.contains("Video") {
                    "Videos"
                } else if let Some(filter) = req.filter.as_deref() {
                    match filter {
                        "songs" => "Songs",
                        "albums" => "Albums",
                        "artists" => "Artists",
                        "playlists" => "Playlists",
                        _ => "Results",
                    }
                } else {
                    "Results"
                }.to_string();

                let mut shelf_items = Vec::new();
                if let Some(items) = shelf["contents"].as_array() {
                    for item in items {
                        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                            let item_type = detect_renderer_type(renderer);
                            match item_type.as_str() {
                                "album" => {
                                    if let Some(album) = parse_search_album_renderer(renderer) {
                                        shelf_items.push(SearchItem::Album(album));
                                    }
                                }
                                "artist" => {
                                    if let Some(artist) = parse_search_artist_renderer(renderer) {
                                        shelf_items.push(SearchItem::Artist(artist));
                                    }
                                }
                                "playlist" => {
                                    if let Some(playlist) = parse_search_playlist_renderer(renderer) {
                                        shelf_items.push(SearchItem::Playlist(playlist));
                                    }
                                }
                                "video" => {
                                    if let Some(track) = parse_search_track_renderer(renderer) {
                                        shelf_items.push(SearchItem::Track(track));
                                    }
                                }
                                "song" => {
                                    if let Some(track) = parse_search_track_renderer(renderer) {
                                        shelf_items.push(SearchItem::Track(track));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }

                if !shelf_items.is_empty() {
                    sections.push(SearchCategorySection {
                        category: category_name,
                        items: shelf_items,
                    });
                }

                if continuation_token.is_none() {
                    if let Some(cont) = shelf["continuations"][0]["nextContinuationData"]["continuation"].as_str() {
                        continuation_token = Some(cont.to_string());
                    }
                }
            }
            // 3. ItemSectionRenderer (Search Summary unfiltered items)
            else if let Some(item_sec) = section.get("itemSectionRenderer") {
                if let Some(inner_contents) = item_sec["contents"].as_array() {
                    for inner in inner_contents {
                        if let Some(renderer) = inner.get("musicResponsiveListItemRenderer") {
                            let item_type = detect_renderer_type(renderer);
                            match item_type.as_str() {
                                "album" => {
                                    if let Some(album) = parse_search_album_renderer(renderer) {
                                        item_section_albums.push(SearchItem::Album(album));
                                    }
                                }
                                "artist" => {
                                    if let Some(artist) = parse_search_artist_renderer(renderer) {
                                        item_section_artists.push(SearchItem::Artist(artist));
                                    }
                                }
                                "playlist" => {
                                    if let Some(playlist) = parse_search_playlist_renderer(renderer) {
                                        item_section_playlists.push(SearchItem::Playlist(playlist));
                                    }
                                }
                                "video" => {
                                    if let Some(track) = parse_search_track_renderer(renderer) {
                                        item_section_videos.push(SearchItem::Track(track));
                                    }
                                }
                                "song" => {
                                    if let Some(track) = parse_search_track_renderer(renderer) {
                                        item_section_songs.push(SearchItem::Track(track));
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    // Append aggregated itemSectionRenderer shelves if present
    if !item_section_songs.is_empty() {
        sections.push(SearchCategorySection {
            category: "Songs".to_string(),
            items: item_section_songs,
        });
    }
    if !item_section_videos.is_empty() {
        sections.push(SearchCategorySection {
            category: "Videos".to_string(),
            items: item_section_videos,
        });
    }
    if !item_section_albums.is_empty() {
        sections.push(SearchCategorySection {
            category: "Albums".to_string(),
            items: item_section_albums,
        });
    }
    if !item_section_artists.is_empty() {
        sections.push(SearchCategorySection {
            category: "Artists".to_string(),
            items: item_section_artists,
        });
    }
    if !item_section_playlists.is_empty() {
        sections.push(SearchCategorySection {
            category: "Playlists".to_string(),
            items: item_section_playlists,
        });
    }

    let search_res = CategorizedSearchResult {
        sections,
        continuation_token,
    };

    Ok(serde_json::to_string(&search_res)?)
}

fn detect_renderer_type(renderer: &serde_json::Value) -> String {
    let flex = renderer["flexColumns"].as_array();
    
    // 1. Check explicit text badge in column 1 (InnerTube's definitive item indicator)
    let badge_type = flex.and_then(|cols| cols.get(1))
        .and_then(|col| col["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array())
        .and_then(|runs| runs.get(0))
        .and_then(|r| r["text"].as_str())
        .map(|s| s.trim().to_lowercase())
        .unwrap_or_default();

    // 2. Check endpoints and browseIds
    let nav_browse_page_type = renderer["navigationEndpoint"]["browseEndpoint"]["browseEndpointContextSupportedConfigs"]["browseEndpointContextMusicConfig"]["pageType"].as_str().unwrap_or("");
    let nav_browse_id = renderer["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str().unwrap_or("");
    
    let col0_browse_page_type = flex.and_then(|cols| cols.get(0))
        .and_then(|col| col["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array())
        .and_then(|runs| runs.get(0))
        .and_then(|r| r["navigationEndpoint"]["browseEndpoint"]["browseEndpointContextSupportedConfigs"]["browseEndpointContextMusicConfig"]["pageType"].as_str())
        .unwrap_or("");
    
    let col0_browse_id = flex.and_then(|cols| cols.get(0))
        .and_then(|col| col["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array())
        .and_then(|runs| runs.get(0))
        .and_then(|r| r["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str())
        .unwrap_or("");

    let watch_endpoint = renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"].clone();
    let music_video_type = watch_endpoint["watchEndpointMusicSupportedConfigs"]["watchEndpointMusicConfig"]["musicVideoType"].as_str()
        .or_else(|| renderer["navigationEndpoint"]["watchEndpoint"]["watchEndpointMusicSupportedConfigs"]["watchEndpointMusicConfig"]["musicVideoType"].as_str());

    // Filter out podcasts, episodes, user profiles, or stations
    if badge_type == "episode" || badge_type == "podcast" || badge_type == "profile" || badge_type == "station"
        || nav_browse_id.starts_with("MPED") || col0_browse_id.starts_with("MPED")
        || nav_browse_id.starts_with("MPSP") || col0_browse_id.starts_with("MPSP")
        || music_video_type == Some("MUSIC_VIDEO_TYPE_PODCAST_EPISODE") {
        return "ignore".to_string();
    }

    if badge_type == "song" {
        return "song".to_string();
    } else if badge_type == "video" {
        if music_video_type == Some("MUSIC_VIDEO_TYPE_OMV") {
            return "video".to_string();
        }
        return "ignore".to_string(); // Discard UGC fan videos from cluttering search
    } else if badge_type == "album" || badge_type == "single" || badge_type == "ep" {
        return "album".to_string();
    } else if badge_type == "artist" {
        return "artist".to_string();
    } else if badge_type.contains("playlist") {
        return "playlist".to_string();
    }

    if nav_browse_id.starts_with("MPREb") || col0_browse_id.starts_with("MPREb") || nav_browse_page_type == "MUSIC_PAGE_TYPE_ALBUM" || col0_browse_page_type == "MUSIC_PAGE_TYPE_ALBUM" {
        "album".to_string()
    } else if nav_browse_page_type == "MUSIC_PAGE_TYPE_ARTIST" || col0_browse_page_type == "MUSIC_PAGE_TYPE_ARTIST" {
        "artist".to_string()
    } else if nav_browse_id.starts_with("VL") || col0_browse_id.starts_with("VL") || nav_browse_page_type == "MUSIC_PAGE_TYPE_PLAYLIST" || col0_browse_page_type == "MUSIC_PAGE_TYPE_PLAYLIST" {
        "playlist".to_string()
    } else if music_video_type == Some("MUSIC_VIDEO_TYPE_ATV") {
        "song".to_string()
    } else if music_video_type == Some("MUSIC_VIDEO_TYPE_OMV") {
        "video".to_string()
    } else {
        "ignore".to_string()
    }
}

fn is_official_music_track(title: &str, artist: &str, music_video_type: Option<&str>) -> bool {
    // 1. If explicit YouTube Music Video Type is available:
    if let Some(mvt) = music_video_type {
        if mvt == "MUSIC_VIDEO_TYPE_UGC" || mvt == "MUSIC_VIDEO_TYPE_PODCAST_EPISODE" {
            return false;
        }
    }

    let title_lower = title.to_lowercase();
    let artist_lower = artist.to_lowercase();

    // 2. Reject obvious non-song video / spam upload keywords
    let banned_title_keywords = [
        "full album", "full ep", "1 hour", "10 hours", "10 hour",
        "nightcore", "slowed + reverb", "slowed and reverb", "slowed & reverb",
        "sped up", "speed up", "8d audio", "tiktok version", "tiktok edit",
        "guitar cover", "drum cover", "piano cover", "bass cover", "instrumental cover",
        "karaoke", "reaction", "parody", "leak", "unreleased snippet", "sleep study relax",
        "relaxing music", "sleep music"
    ];

    for kw in &banned_title_keywords {
        if title_lower.contains(kw) {
            return false;
        }
    }

    // 3. Reject non-artist channel bylines
    let banned_artist_keywords = [
        "lyrics", "nightcore", "vibes", "edits", "audio hub", "remix hub"
    ];

    for kw in &banned_artist_keywords {
        if artist_lower == *kw || (artist_lower.contains(kw) && !artist_lower.contains("- topic")) {
            return false;
        }
    }

    true
}
fn parse_search_track_renderer(renderer: &serde_json::Value) -> Option<TrackResult> {
    let cols = renderer["flexColumns"].as_array()?;
    let col0 = cols.get(0)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array()?;
    let title = col0.get(0)?["text"].as_str()?.to_string();
    
    let video_id = renderer["playlistItemData"]["videoId"].as_str()
        .or_else(|| col0.get(0)?["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
        .or_else(|| renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
        .unwrap_or("").to_string();

    if video_id.is_empty() {
        return None;
    }

    let mut artist = String::new();
    let mut album: Option<String> = None;
    let mut duration_ms: Option<u64> = None;
    let mut plays: Option<String> = None;

    if let Some(col1) = cols.get(1) {
        if let Some(runs) = col1["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array() {
            // Group runs by bullet / delimiter (• or \u{2022} or ·)
            let mut segments: Vec<Vec<&serde_json::Value>> = Vec::new();
            let mut current_segment = Vec::new();

            for r in runs {
                let text = r["text"].as_str().unwrap_or("").trim();
                if text == "•" || text == "\u{2022}" || text == "·" {
                    if !current_segment.is_empty() {
                        segments.push(current_segment);
                        current_segment = Vec::new();
                    }
                } else if !text.is_empty() {
                    current_segment.push(r);
                }
            }
            if !current_segment.is_empty() {
                segments.push(current_segment);
            }

            // Segment 0 is the Artist(s) segment (filter out leading "Song" or "Video" badge if present)
            if let Some(art_seg) = segments.get(0) {
                let mut artist_parts = Vec::new();
                for r in art_seg {
                    let t = r["text"].as_str().unwrap_or("").trim();
                    let low = t.to_lowercase();
                    if low != "song" && low != "video" {
                        artist_parts.push(t);
                    }
                }
                artist = artist_parts.join(" ");
            }

            // Subsequent segments (Album, Plays/Views, Duration)
            for seg in segments.iter().skip(1) {
                for r in seg {
                    let t = r["text"].as_str().unwrap_or("").trim();
                    if t.is_empty() || t == "&" || t == "." || t == "," || t == "-" || t == "•" {
                        continue;
                    }
                    if let Some(d) = parse_duration_ms(t) {
                        duration_ms = Some(d);
                    } else if t.to_lowercase().contains("play") || t.to_lowercase().contains("view") {
                        plays = Some(t.to_string());
                    } else if album.is_none() {
                        album = Some(t.to_string());
                    }
                }
            }
        }
    }

    if duration_ms.is_none() {
        if let Some(fixed_cols) = renderer["fixedColumns"].as_array() {
            if let Some(fc0) = fixed_cols.get(0) {
                if let Some(runs) = fc0["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"].as_array() {
                    if let Some(dur_text) = runs.get(0).and_then(|r| r["text"].as_str()) {
                        duration_ms = parse_duration_ms(dur_text);
                    }
                }
            }
        }
    }

    let music_video_type = col0.get(0)
        .and_then(|c| c["navigationEndpoint"]["watchEndpoint"]["watchEndpointMusicSupportedConfigs"]["watchEndpointMusicConfig"]["musicVideoType"].as_str())
        .or_else(|| renderer["overlay"]["musicItemThumbnailOverlayRenderer"]["content"]["musicPlayButtonRenderer"]["playNavigationEndpoint"]["watchEndpoint"]["watchEndpointMusicSupportedConfigs"]["watchEndpointMusicConfig"]["musicVideoType"].as_str());

    if !is_official_music_track(&title, &artist, music_video_type) {
        return None;
    }

    if artist.ends_with(" - Topic") {
        artist = artist.trim_end_matches(" - Topic").to_string();
    }

    let cover = upscale_yt_art(renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"][0]["url"].as_str());

    Some(TrackResult {
        id: video_id,
        title,
        artist,
        album,
        cover_art_url: cover,
        stream_url: None,
        quality_hint: None,
        duration_ms,
        plays,
    })
}

fn parse_search_album_renderer(renderer: &serde_json::Value) -> Option<AlbumItem> {
    let cols = renderer["flexColumns"].as_array()?;
    let col0 = cols.get(0)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array()?;
    let title = col0.get(0)?["text"].as_str()?.to_string();
    let browse_id = col0.get(0)?["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
        .or_else(|| renderer["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str())
        .unwrap_or("").to_string();

    let mut artist = String::new();
    let mut year = None;

    if let Some(col1) = cols.get(1) {
        if let Some(runs) = col1["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array() {
            let raw_parts: Vec<&str> = runs.iter().filter_map(|r| r["text"].as_str()).filter(|t| *t != " • " && *t != " \u{2022} ").collect();
            
            // Strip leading type badges like "Album", "Single", "EP"
            let text_parts: Vec<&str> = raw_parts.into_iter()
                .filter(|p| {
                    let low = p.trim().to_lowercase();
                    low != "album" && low != "single" && low != "ep" && low != "playlist"
                })
                .collect();

            if let Some(a) = text_parts.get(0) {
                artist = a.to_string();
            }
            if let Some(y) = text_parts.get(1) {
                year = Some(y.to_string());
            }
        }
    }

    let cover = upscale_yt_art(renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"][0]["url"].as_str());

    Some(AlbumItem {
        id: browse_id,
        title,
        artist,
        cover_art_url: cover,
        year,
    })
}

fn parse_search_artist_renderer(renderer: &serde_json::Value) -> Option<ArtistItem> {
    let cols = renderer["flexColumns"].as_array()?;
    let col0 = cols.get(0)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array()?;
    let name = col0.get(0)?["text"].as_str()?.to_string();
    let browse_id = col0.get(0)?["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
        .or_else(|| renderer["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str())
        .unwrap_or("").to_string();

    let mut subscribers = None;
    if let Some(col1) = cols.get(1) {
        if let Some(runs) = col1["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array() {
            subscribers = runs.iter().filter_map(|r| r["text"].as_str()).next().map(|s| s.to_string());
        }
    }

    let cover = upscale_yt_art(renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"][0]["url"].as_str());

    Some(ArtistItem {
        id: browse_id,
        name,
        avatar_url: cover,
        subscribers,
    })
}

fn parse_search_playlist_renderer(renderer: &serde_json::Value) -> Option<PlaylistItem> {
    let cols = renderer["flexColumns"].as_array()?;
    let col0 = cols.get(0)?["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array()?;
    let title = col0.get(0)?["text"].as_str()?.to_string();
    let browse_id = col0.get(0)?["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str()
        .or_else(|| renderer["navigationEndpoint"]["browseEndpoint"]["browseId"].as_str())
        .unwrap_or("").to_string();

    let mut curator = None;
    let mut track_count = None;

    if let Some(col1) = cols.get(1) {
        if let Some(runs) = col1["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"].as_array() {
            let parts: Vec<&str> = runs.iter().filter_map(|r| r["text"].as_str()).filter(|t| *t != " • " && *t != " \u{2022} ").collect();
            if let Some(c) = parts.get(0) {
                curator = Some(c.to_string());
            }
            if let Some(tc) = parts.get(1) {
                track_count = tc.split_whitespace().next().and_then(|n| n.parse::<u32>().ok());
            }
        }
    }

    let cover = upscale_yt_art(renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"][0]["url"].as_str());

    Some(PlaylistItem {
        id: browse_id,
        title,
        author: curator,
        item_count: track_count,
        cover_art_url: cover,
    })
}

#[plugin_fn]
pub fn search(input: String) -> FnResult<String> {
    do_search(input)
}

#[plugin_fn]
pub fn search_categorized(input: String) -> FnResult<String> {
    do_search_categorized(input)
}

#[plugin_fn]
pub fn resolve(input: String) -> FnResult<String> {
    let track_id: String = serde_json::from_str(&input)?;
    unsafe { host_log(format!("WASM resolving track: {}", track_id))? };

    let fresh_init = init_botguard()?;
    let vd = get_visitor_data()?;

    // The visitorData-bound token is the *session* token: BotGuard requires it be minted
    // exactly once, before any other token, immediately after the minter is (re)created —
    // and it must be reused across tracks, not re-minted every resolve() call.
    let cached_vd = get_storage("session_potoken_vd").unwrap_or_default();
    let mut session_potoken = if fresh_init || cached_vd != vd {
        String::new()
    } else {
        get_storage("session_potoken").unwrap_or_default()
    };
    if session_potoken.is_empty() {
        session_potoken = match generate_po_token(&vd) {
            Ok(tok) => tok,
            Err(_) => {
                unsafe { host_log("session_potoken mint failed, re-initializing BotGuard...".to_string())? };
                let _ = extism_pdk::var::set("bg_init", "false");
                init_botguard()?;
                generate_po_token(&vd).unwrap_or_default()
            }
        };
        set_storage("session_potoken", &session_potoken)?;
        set_storage("session_potoken_vd", &vd)?;
        unsafe { host_log(format!("Minted session-bound poToken (first 20 chars): {}", session_potoken.chars().take(20).collect::<String>()))? };
    }

    // The videoId-bound token goes on the CDN stream URL as `pot=`, minted fresh per track.
    let video_potoken = match generate_po_token(&track_id) {
        Ok(tok) => tok,
        Err(_) => {
            unsafe { host_log("video_potoken mint failed, re-initializing BotGuard...".to_string())? };
            let _ = extism_pdk::var::set("bg_init", "false");
            init_botguard()?;
            generate_po_token(&track_id).unwrap_or_default()
        }
    };
    unsafe { host_log(format!("Minted videoId-bound poToken (first 20 chars): {}", video_potoken.chars().take(20).collect::<String>()))? };

    let sts: i64 = get_or_refresh_sts();

    let mut stream_url = String::new();
    let mut quality_hint = Some("Medium".to_string());
    let mut duration_sec: u64 = 0;
    let mut selected_client: Option<YtClient> = None;

    for client in fallback_clients() {
        unsafe { host_log(format!("[RESOLVE] Trying client {} v{}", client.name, client.version))? };

        let po_token_for_body = if client.needs_potoken { Some(session_potoken.as_str()) } else { None };
        let sts_for_body = if client.needs_sts { Some(sts) } else { None };
        let body = build_player_body(&client, &track_id, &vd, po_token_for_body, sts_for_body);

        let json = match request_player(&client, &body, &vd) {
            Ok(j) => j,
            Err(e) => {
                unsafe { host_log(format!("[RESOLVE] {} request failed: {:?}", client.name, e))? };
                continue;
            }
        };

        let status = json["playabilityStatus"]["status"].as_str().unwrap_or("");
        if status != "OK" {
            let reason = json["playabilityStatus"]["reason"].as_str().unwrap_or("");
            unsafe { host_log(format!("[RESOLVE] {} not playable: status={} reason={}", client.name, status, reason))? };
            continue;
        }

        let formats = match json["streamingData"]["adaptiveFormats"].as_array() {
            Some(f) => f,
            None => {
                unsafe { host_log(format!("[RESOLVE] {} OK but no adaptiveFormats", client.name))? };
                continue;
            }
        };

        // Echo now supports Opus decoding! We prefer Opus for better quality at similar bitrates,
        // falling back to AAC, then ranking by bitrate.
        let player_hash = get_storage("yt_player_hash").unwrap_or_default();
        let mut best: Option<(&serde_json::Value, u8, i64, Option<(String, String, String)>)> = None;
        for format in formats {
            let mime = format["mimeType"].as_str().unwrap_or("");
            if !mime.starts_with("audio/") {
                continue;
            }

            let direct_url = format["url"].as_str();
            let cipher_data = if direct_url.is_none() {
                if let Some(sc) = format["signatureCipher"].as_str().or_else(|| format["cipher"].as_str()) {
                    parse_signature_cipher(sc)
                } else {
                    None
                }
            } else {
                None
            };

            if direct_url.is_none() && cipher_data.is_none() {
                continue;
            }
            
            let format_score = if mime.contains("opus") {
                2
            } else if mime.starts_with("audio/mp4") {
                1
            } else {
                0
            };
            
            let bitrate = format["bitrate"].as_i64().unwrap_or(0);
            let should_replace = match best {
                None => true,
                Some((_, best_score, best_bitrate, _)) => (format_score, bitrate) > (best_score, best_bitrate),
            };
            if should_replace {
                best = Some((format, format_score, bitrate, cipher_data));
            }
        }

        if let Some((format, _, _, cipher_data)) = best {
            if let Some((obfuscated_s, sp, base_url)) = cipher_data {
                unsafe { host_log(format!("[RESOLVE] Selected {} — audio format with signatureCipher, deciphering...", client.name))? };
                let deciphered_s = deobfuscate_signature(&obfuscated_s, &player_hash);
                let sep = if base_url.contains('?') { "&" } else { "?" };
                stream_url = format!("{}{}{}={}", base_url, sep, sp, url_encode_component(&deciphered_s));
            } else {
                stream_url = format["url"].as_str().unwrap_or("").to_string();
                unsafe { host_log(format!("[RESOLVE] Selected {} — audio format with direct URL", client.name))? };
            }
            quality_hint = format["audioQuality"].as_str().map(|s| s.to_string());
            duration_sec = json["videoDetails"]["lengthSeconds"].as_str().unwrap_or("0").parse().unwrap_or(0);
            selected_client = Some(client);
            break;
        } else {
            unsafe { host_log(format!("[RESOLVE] {} OK but no usable audio formats found", client.name))? };
        }
    }

    let mut out_headers = HashMap::new();
    if let Some(ref client) = selected_client {
        out_headers.insert("User-Agent".to_string(), client.user_agent.to_string());
        if client.needs_potoken {
            // Transform the 'n' parameter to avoid 40-60 kbps CDN bandwidth throttling
            let player_hash = get_storage("yt_player_hash").unwrap_or_default();
            if !player_hash.is_empty() {
                stream_url = apply_n_transform_to_url(&stream_url, &player_hash);
            }

            // Only clients that use poTokens (WEB_REMIX et al.) also need the CDN pot= param,
            // and their CDN validates a matching Origin/Referer pair.
            if !video_potoken.is_empty() {
                let sep = if stream_url.contains('?') { "&" } else { "?" };
                stream_url.push_str(&format!("{}pot={}", sep, url_encode_component(&video_potoken)));
            }
            out_headers.insert("Origin".to_string(), "https://music.youtube.com".to_string());
            out_headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());
        }
    } else {
        unsafe { host_log("[RESOLVE] All fallback clients exhausted; no playable stream found".to_string())? };
    }

    unsafe { host_log(format!("[RESOLVE FINAL RESULT] stream_url length: {}, quality: {:?}", stream_url.len(), quality_hint))? };

    let resolved = ResolvedTrack {
        stream_url,
        quality_hint,
        duration_ms: Some(duration_sec * 1000),
        headers: Some(out_headers),
    };

    Ok(serde_json::to_string(&resolved)?)
}

#[plugin_fn]
pub fn get_modules() -> FnResult<String> {
    let mods = vec![
        ProviderModule {
            id: "spotlight".to_string(),
            name: "Featured Spotlight".to_string(),
            layout: "banner".to_string(),
        },
        ProviderModule {
            id: "categories".to_string(),
            name: "Browse Categories".to_string(),
            layout: "grid4".to_string(),
        },
        ProviderModule {
            id: "trending".to_string(),
            name: "Top Global Tracks".to_string(),
            layout: "ledger".to_string(),
        },
        ProviderModule {
            id: "new_releases".to_string(),
            name: "New Releases".to_string(),
            layout: "grid2x2".to_string(),
        },
        ProviderModule {
            id: "trending_albums".to_string(),
            name: "Trending Albums".to_string(),
            layout: "carousel".to_string(),
        },
    ];
    Ok(serde_json::to_string(&mods)?)
}


#[plugin_fn]
pub fn warmup(_input: String) -> FnResult<String> {
    let _ = unsafe { host_log("[WARMUP] Starting YouTube WASM prewarm...".to_string()) };

    // 1. Pre-warm BotGuard JNN & visitorData
    let vd = get_visitor_data().unwrap_or_default();
    if let Err(e) = init_botguard() {
        let _ = unsafe { host_log(format!("[WARMUP] BotGuard init skipped/failed: {:?}", e)) };
    } else if !vd.is_empty() {
        let _ = generate_po_token(&vd);
    }

    // 2. Pre-warm Player JS & dynamic STS
    let sts = get_or_refresh_sts();
    let player_hash = get_storage("yt_player_hash").unwrap_or_default();
    if !player_hash.is_empty() {
        let _ = init_sandbox_player_js_if_needed(&player_hash);
        let _ = unsafe { host_log(format!("[WARMUP] Cached dynamic STS={} and player hash={}", sts, player_hash)) };
    }

    let _ = unsafe { host_log("[WARMUP] YouTube WASM prewarm completed successfully".to_string()) };
    Ok(serde_json::json!({"status": "ready"}).to_string())
}

fn parse_category_page_shelves(json: &serde_json::Value) -> Vec<ModuleItem> {
    let mut items = Vec::new();
    let sections = json.pointer("/contents/singleColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents")
        .or_else(|| json.pointer("/contents/sectionListRenderer/contents"))
        .or_else(|| json.pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents"))
        .and_then(|v| v.as_array());

    if let Some(sections_arr) = sections {
        for sec in sections_arr {
            // 1. Music Carousel Shelf (Thematic Playlists / Mixes / Albums)
            if let Some(carousel) = sec.get("musicCarouselShelfRenderer") {
                let header = carousel.pointer("/header/musicCarouselShelfBasicHeaderRenderer/title/runs/0/text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Featured Mixes")
                    .to_string();

                let mut shelf_playlists = Vec::new();

                if let Some(contents) = carousel["contents"].as_array() {
                    for item in contents {
                        if let Some(r) = item.get("musicTwoRowItemRenderer") {
                            let nav = &r["navigationEndpoint"]["browseEndpoint"];
                            let browse_id = nav["browseId"].as_str().unwrap_or("");
                            let title = r["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            if title.is_empty() { continue; }

                            let subtitle = r["subtitle"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let cover = r["thumbnailRenderer"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"].as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|t| t["url"].as_str())
                                .map(|s| s.to_string());

                            if browse_id.starts_with("MPREb_") || browse_id.starts_with("FEmusic_album") {
                                items.push(ModuleItem::Album(AlbumItem {
                                    id: browse_id.to_string(),
                                    title,
                                    artist: subtitle,
                                    cover_art_url: cover,
                                    year: None,
                                }));
                            } else {
                                shelf_playlists.push(PlaylistItem {
                                    id: browse_id.to_string(),
                                    title,
                                    author: Some(subtitle),
                                    cover_art_url: cover,
                                    item_count: None,
                                });
                            }
                        }
                    }
                }

                if !shelf_playlists.is_empty() {
                    items.push(ModuleItem::Shelf(CategoryShelf {
                        title: header,
                        items: shelf_playlists,
                    }));
                }
            }

            // 2. Playlist / Track Shelf (Top Singles)
            if let Some(shelf) = sec.get("musicPlaylistShelfRenderer").or_else(|| sec.get("musicShelfRenderer")) {
                if let Some(contents) = shelf["contents"].as_array() {
                    let mut rank = 1;
                    for item in contents {
                        if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                            let vid = renderer["playlistItemData"]["videoId"].as_str()
                                .or_else(|| renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .or_else(|| renderer["navigationEndpoint"]["watchEndpoint"]["videoId"].as_str())
                                .unwrap_or("").to_string();
                            if vid.is_empty() { continue; }

                            let raw_title = renderer["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let raw_artist = renderer["flexColumns"].get(1)
                                .and_then(|c| c["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .unwrap_or("").to_string();
                            let (title, artist) = clean_title_and_artist(&raw_title, &raw_artist);
                            let cover = renderer["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"].as_array()
                                .and_then(|arr| arr.last())
                                .and_then(|t| t["url"].as_str())
                                .map(|s| s.to_string());
                            let dur_ms = renderer.get("fixedColumns")
                                .and_then(|fc| fc.get(0))
                                .and_then(|c| c["musicResponsiveListItemFixedColumnRenderer"]["text"]["runs"][0]["text"].as_str())
                                .and_then(parse_duration_ms);

                            items.push(ModuleItem::Track(TrackResult {
                                id: vid,
                                title,
                                artist,
                                album: None,
                                cover_art_url: cover,
                                stream_url: None,
                                quality_hint: Some(format!("#{}", rank)),
                                duration_ms: dur_ms,
                                plays: None,
                            }));
                            rank += 1;
                        }
                    }
                }
            }
        }
    }
    items
}

#[plugin_fn]
pub fn fetch_module(input: String) -> FnResult<String> {
    let module_id: String = serde_json::from_str(&input).unwrap_or(input);
    let _ = unsafe { host_log(format!("[EXPLORE] Fetching module: {}", module_id)) };

    let items = match module_id.as_str() {
        "spotlight" => {
            match browse_innertube("FEmusic_explore", None, None) {
                Ok(json) => {
                    let parsed = parse_editorial_spotlights(&json);
                    if !parsed.is_empty() {
                        parsed
                    } else {
                        vec![default_spotlight(), default_secondary_spotlight()]
                    }
                }
                Err(_) => Vec::new(),
            }
        }
        "categories" | "moods_genres" => {
            let mut res = Vec::new();
            if let Ok(json) = browse_innertube("FEmusic_moods_and_genres", None, None) {
                res = parse_moods_and_genres(&json);
            }
            if res.is_empty() {
                if let Ok(json) = browse_innertube("FEmusic_explore", None, None) {
                    res = parse_moods_and_genres(&json);
                }
            }
            res
        }
        "trending" | "charts_top" => {
            // Global Top 50 / Top 100 Chart
            if let Ok(pl_json) = browse_innertube("VLPL4fGSI1pDJn69On1f-8NAvX_CYlx7QyZc", None, Some("US")) {
                let parsed = parse_playlist_shelf_tracks(&pl_json);
                if !parsed.is_empty() { parsed } else { fallback_search_tracks("Top 50 Global Hits") }
            } else {
                fallback_search_tracks("Top 50 Global Hits")
            }
        }
        "charts_viral" | "viral" => {
            // Trending Viral / Daily Top Music Videos
            let direct = fetch_chart_carousel_playlist(None, 1, Some("Trending"));
            if !direct.is_empty() {
                direct
            } else {
                fetch_chart_carousel_playlist(None, 0, None)
            }
        }
        m if m.starts_with("charts_regional_") => {
            let country = &m["charts_regional_".len()..];
            // Fetch the Top 100 / Top Songs regional chart (Item 2 or preferred "Top 100" / "Top Songs"), separating from Trending 20 (Item 0)
            let direct = fetch_chart_carousel_playlist(Some(country), 2, Some("Top 100"));
            if !direct.is_empty() {
                direct
            } else {
                let fallback_direct = fetch_chart_carousel_playlist(Some(country), 1, None);
                if !fallback_direct.is_empty() {
                    fallback_direct
                } else {
                    fallback_search_tracks(&format!("Top 50 Songs {}", country))
                }
            }
        }
        "new_releases" => {
            let mut res = Vec::new();
            if let Ok(json) = browse_innertube("FEmusic_new_releases_albums", None, None) {
                res = parse_new_releases_albums(&json);
            }
            if res.is_empty() {
                if let Ok(json) = browse_innertube("FEmusic_new_releases", None, None) {
                    res = parse_new_releases_albums(&json);
                }
            }
            if res.is_empty() {
                if let Ok(json) = browse_innertube("FEmusic_explore", None, None) {
                    res = parse_new_releases_albums(&json);
                }
            }
            res
        }

        "trending_albums" => {
            let mut res = Vec::new();
            if let Ok(json) = browse_innertube("FEmusic_new_releases_albums", None, None) {
                res = parse_trending_albums(&json);
            }
            if res.is_empty() {
                if let Ok(json) = browse_innertube("FEmusic_explore", None, None) {
                    res = parse_trending_albums(&json);
                }
            }
            res
        }
        "trending_artists" => {
            match browse_innertube("FEmusic_charts", None, None) {
                Ok(json) => {
                    parse_trending_artists(&json)
                }
                Err(_) => Vec::new(),
            }
        }
        "top_videos" => {
            match browse_innertube("FEmusic_charts", None, None) {
                Ok(json) => {
                    parse_charts_tracks(&json, None)
                }
                Err(_) => Vec::new(),
            }
        }
        m if m.starts_with("FEmusic_moods_and_genres_category") || m.contains(":") => {
            let parts: Vec<&str> = m.splitn(2, ":").collect();
            let browse_id = parts[0];
            let params = parts.get(1).copied();
            
            let mut res = Vec::new();
            if let Ok(json) = browse_innertube(browse_id, params, None) {
                res = parse_category_page_shelves(&json);
            }
            if res.is_empty() {
                res = fallback_search_tracks(parts.get(1).unwrap_or(&browse_id));
            }
            res
        }
        _ => fallback_search_tracks(&module_id),
    };

    let items = sanitize_module_items(items);
    let data = ModuleData { items };
    Ok(serde_json::to_string(&data)?)
}

fn fallback_search_tracks(query: &str) -> Vec<ModuleItem> {
    let json_input = serde_json::to_string(query).unwrap_or_else(|_| format!("\"{}\"", query));
    if let Ok(res) = do_search(json_input) {
        let tracks: Vec<TrackResult> = serde_json::from_str(&res).unwrap_or_default();
        tracks.into_iter().map(ModuleItem::Track).collect()
    } else {
        Vec::new()
    }
}




fn default_secondary_spotlight() -> ModuleItem {
    ModuleItem::Spotlight(EditorialSpotlight {
        id: "RDCLAK5uy_synthwave_spotlight".into(),
        title: "Neon Horizon: Tokyo 2099".into(),
        artist: "GUNSHIP & The Midnight".into(),
        cover_art_url: Some("https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=1200&q=85".into()),
        description: Some("Editorial Pick • Cyberpunk Synthwave".into()),
        release_year: Some("2026".into()),
        track_count: Some(14),
    })
}

fn default_spotlight() -> ModuleItem {
    ModuleItem::Spotlight(EditorialSpotlight {
        id: "RDCLAK5uy_editorial_album_of_month".into(),
        title: "Echoes of Eternity".into(),
        artist: "Kavinsky & Daft Punk".into(),
        cover_art_url: Some("https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=1200&q=85".into()),
        description: Some("Album of the Month • Lossless Master".into()),
        release_year: Some("2026".into()),
        track_count: Some(10),
    })
}

fn default_playlists() -> Vec<ModuleItem> {
    vec![
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn69On1f-8NAvX_CYlx7QyZc".into(),
            title: "Deep Work & Flow State".into(),
            author: Some("Curated Electronic • Minimalist Focus".into()),
            item_count: Some(50),
            cover_art_url: Some("https://images.unsplash.com/photo-1518770660439-4636190af475?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn6-TcxZ5z0dZ5q5K5d5z0".into(),
            title: "Analog Synth Explorations".into(),
            author: Some("Modular Synthesis & Vintage Electronic".into()),
            item_count: Some(45),
            cover_art_url: Some("https://images.unsplash.com/photo-1598488035139-bdbb2231ce04?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn68P0rX_e3Z9pE-JjU397M6".into(),
            title: "Midnight Tokyo City Pop".into(),
            author: Some("80s Japanese Groove & Nostalgia".into()),
            item_count: Some(60),
            cover_art_url: Some("https://images.unsplash.com/photo-1509198397868-475647b2a1e5?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn6bXp0B2uQYQJ1r132_X".into(),
            title: "Audiophile Acoustic Sessions".into(),
            author: Some("Pure Master Recording • Lossless Studio".into()),
            item_count: Some(38),
            cover_art_url: Some("https://images.unsplash.com/photo-1511671782779-c97d3d27a1d4?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn67z1mXq1Y2Z3A4B5C6D".into(),
            title: "Late Night Drive & Cyberpunk".into(),
            author: Some("Darksynth, French Touch & Nu-Disco".into()),
            item_count: Some(55),
            cover_art_url: Some("https://images.unsplash.com/photo-1508700115892-45ecd05ae2ad?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn65a2b3c4d5e6f7g8h9i".into(),
            title: "Vinyl Jazz Vault & Neo-Soul".into(),
            author: Some("Late Night Warmth & Analog Grooves".into()),
            item_count: Some(42),
            cover_art_url: Some("https://images.unsplash.com/photo-1514525253161-7a46d19cd819?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn63j4k5l6m7n8o9p0q1r".into(),
            title: "High-Octane Industrial Workout".into(),
            author: Some("Heavy Basslines & Pure Kinetic Energy".into()),
            item_count: Some(50),
            cover_art_url: Some("https://images.unsplash.com/photo-1517838277536-f5f99be501cd?q=80&w=800&auto=format&fit=crop".into()),
        }),
        ModuleItem::Playlist(PlaylistItem {
            id: "VLPL4fGSI1pDJn61s2t3u4v5w6x7y8z9a".into(),
            title: "Cinematic Orchestral Masterworks".into(),
            author: Some("Epic Film Scores & Modern Chamber".into()),
            item_count: Some(40),
            cover_art_url: Some("https://images.unsplash.com/photo-1518709268805-4e9042af9f23?q=80&w=800&auto=format&fit=crop".into()),
        }),
    ]
}

fn default_artists() -> Vec<ModuleItem> {
    vec![
        ModuleItem::Artist(ArtistItem {
            id: "UC_artist_the_weeknd".into(),
            name: "The Weeknd".into(),
            avatar_url: Some("https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=400&q=80".into()),
            subscribers: Some("35M subscribers".into()),
        }),
        ModuleItem::Artist(ArtistItem {
            id: "UC_artist_taylor_swift".into(),
            name: "Taylor Swift".into(),
            avatar_url: Some("https://images.unsplash.com/photo-1517841905240-472988babdf9?w=400&q=80".into()),
            subscribers: Some("58M subscribers".into()),
        }),
        ModuleItem::Artist(ArtistItem {
            id: "UC_artist_drake".into(),
            name: "Drake".into(),
            avatar_url: Some("https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?w=400&q=80".into()),
            subscribers: Some("29M subscribers".into()),
        }),
    ]
}

fn default_genres() -> Vec<ModuleItem> {
    vec![
        ModuleItem::Genre(GenreItem { id: "chill".into(), title: "Chill & Downtempo".into(), endpoint_params: None, color_hex: Some("#5B8C96".into()) }),
        ModuleItem::Genre(GenreItem { id: "focus".into(), title: "Deep Focus".into(), endpoint_params: None, color_hex: Some("#2D4263".into()) }),
        ModuleItem::Genre(GenreItem { id: "electronic".into(), title: "Electronic & Synth".into(), endpoint_params: None, color_hex: Some("#19A7CE".into()) }),
        ModuleItem::Genre(GenreItem { id: "ambient".into(), title: "Ambient & Space".into(), endpoint_params: None, color_hex: Some("#16A085".into()) }),
        ModuleItem::Genre(GenreItem { id: "workout".into(), title: "Workout & Drive".into(), endpoint_params: None, color_hex: Some("#C84B31".into()) }),
        ModuleItem::Genre(GenreItem { id: "jazz".into(), title: "Jazz & Neo-Soul".into(), endpoint_params: None, color_hex: Some("#D4A86E".into()) }),
        ModuleItem::Genre(GenreItem { id: "classical".into(), title: "Modern Classical".into(), endpoint_params: None, color_hex: Some("#8B7EA8".into()) }),
        ModuleItem::Genre(GenreItem { id: "hiphop".into(), title: "Hip-Hop & Beats".into(), endpoint_params: None, color_hex: Some("#C0392B".into()) }),
        ModuleItem::Genre(GenreItem { id: "rock".into(), title: "Indie & Rock".into(), endpoint_params: None, color_hex: Some("#2C3E50".into()) }),
        ModuleItem::Genre(GenreItem { id: "lofi".into(), title: "Lo-Fi Masterclass".into(), endpoint_params: None, color_hex: Some("#5E9E68".into()) }),
        ModuleItem::Genre(GenreItem { id: "night_drive".into(), title: "Late Night Drive".into(), endpoint_params: None, color_hex: Some("#C87D55".into()) }),
        ModuleItem::Genre(GenreItem { id: "acoustic".into(), title: "Acoustic Folk".into(), endpoint_params: None, color_hex: Some("#C5A059".into()) }),
        ModuleItem::Genre(GenreItem { id: "synthwave".into(), title: "Synthwave & Retro".into(), endpoint_params: None, color_hex: Some("#E26868".into()) }),
        ModuleItem::Genre(GenreItem { id: "cinematic".into(), title: "Film & Soundtracks".into(), endpoint_params: None, color_hex: Some("#667C8A".into()) }),
        ModuleItem::Genre(GenreItem { id: "sleep".into(), title: "Sleep & Rain".into(), endpoint_params: None, color_hex: Some("#3F4E4F".into()) }),
        ModuleItem::Genre(GenreItem { id: "party".into(), title: "Party & Club".into(), endpoint_params: None, color_hex: Some("#F38181".into()) }),
        ModuleItem::Genre(GenreItem { id: "feel_good".into(), title: "Feel Good & Uplifting".into(), endpoint_params: None, color_hex: Some("#F8B195".into()) }),
        ModuleItem::Genre(GenreItem { id: "rnb".into(), title: "R&B & Soul".into(), endpoint_params: None, color_hex: Some("#9B59B6".into()) }),
        ModuleItem::Genre(GenreItem { id: "metal".into(), title: "Metal & Heavy".into(), endpoint_params: None, color_hex: Some("#34495E".into()) }),
        ModuleItem::Genre(GenreItem { id: "global_top".into(), title: "Global Top".into(), endpoint_params: None, color_hex: Some("#B58E62".into()) }),
    ]
}

fn extract_youtube_video_id(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if !trimmed.contains("youtube.com") && !trimmed.contains("youtu.be") {
        return None;
    }
    
    if let Some(pos) = trimmed.find("youtu.be/") {
        let after = &trimmed[pos + 9..];
        let id: String = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if id.len() == 11 {
            return Some(id);
        }
    }
    
    if let Some(pos) = trimmed.find("v=") {
        let after = &trimmed[pos + 2..];
        let id: String = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if id.len() == 11 {
            return Some(id);
        }
    }
    
    if let Some(pos) = trimmed.find("/embed/") {
        let after = &trimmed[pos + 7..];
        let id: String = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if id.len() == 11 {
            return Some(id);
        }
    }
    
    if let Some(pos) = trimmed.find("/v/") {
        let after = &trimmed[pos + 3..];
        let id: String = after.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
        if id.len() == 11 {
            return Some(id);
        }
    }
    
    None
}

#[plugin_fn]
pub fn resolve_url(input: String) -> FnResult<String> {
    let url: String = serde_json::from_str(&input).unwrap_or(input);
    let video_id = match extract_youtube_video_id(&url) {
        Some(id) => id,
        None => {
            let res: Option<TrackResult> = None;
            return Ok(serde_json::to_string(&res)?);
        }
    };
    
    unsafe { host_log(format!("[RESOLVE_URL] Detected YouTube video ID: {}", video_id))? };
    
    let vd = get_visitor_data().unwrap_or_default();
    let sts = get_or_refresh_sts();
    
    let mut track_title = format!("YouTube Track ({})", video_id);
    let mut track_artist = "YouTube".to_string();
    let cover_art_url = Some(format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id));
    let mut duration_ms = None;
    
    for client in fallback_clients() {
        let body = build_player_body(&client, &video_id, &vd, None, Some(sts));
        if let Ok(json) = request_player(&client, &body, &vd) {
            if let Some(details) = json.get("videoDetails") {
                if let Some(t) = details.get("title").and_then(|v| v.as_str()) {
                    track_title = t.to_string();
                }
                if let Some(a) = details.get("author").and_then(|v| v.as_str()) {
                    track_artist = a.to_string();
                }
                if let Some(l) = details.get("lengthSeconds").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()) {
                    duration_ms = Some(l * 1000);
                }
                break;
            }
        }
    }
    
    let result = TrackResult {
        id: video_id,
        title: track_title,
        artist: track_artist,
        album: None,
        cover_art_url,
        stream_url: None,
        quality_hint: Some("Direct URL".to_string()),
        duration_ms,
                                plays: None,
    };
    
    Ok(serde_json::to_string(&Some(result))?)
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

#[plugin_fn]
pub fn browse_album(input: String) -> FnResult<String> {
    let browse_id: String = serde_json::from_str(&input).unwrap_or(input);
    unsafe { host_log(format!("WASM browse_album for: {}", browse_id))? };

    let vd = get_visitor_data().unwrap_or_default();
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.01.00",
                "visitorData": vd
            }
        },
        "browseId": browse_id
    });

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());

    let res = do_http("POST", "https://music.youtube.com/youtubei/v1/browse", Some(headers), Some(body.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&res.body).unwrap_or_default();

    let mut title = String::new();
    let mut artist = String::new();
    let mut year = None;
    let mut description = None;
    let mut cover_art_url = None;
    let mut track_count = None;
    let mut tracks = Vec::new();

    // 1. Header parsing
    let header_opt = json["header"]["musicDetailHeaderRenderer"].as_object()
        .or_else(|| json["header"]["musicResponsiveHeaderRenderer"].as_object())
        .or_else(|| json["header"]["musicEditablePlaylistDetailHeaderRenderer"]["header"]["musicDetailHeaderRenderer"].as_object())
        .or_else(|| json["header"]["musicHeaderRenderer"].as_object());

    if let Some(header) = header_opt {
        let header_val = serde_json::Value::Object(header.clone());
        if let Some(t) = header_val["title"]["runs"][0]["text"].as_str() {
            title = t.to_string();
        }

        if let Some(sub_runs) = header_val["subtitle"]["runs"].as_array() {
            let parts: Vec<&str> = sub_runs.iter().filter_map(|r| r["text"].as_str()).filter(|t| *t != " • " && *t != " \u{2022} ").collect();
            let non_badge: Vec<&str> = parts.into_iter()
                .filter(|p| {
                    let low = p.trim().to_lowercase();
                    low != "album" && low != "single" && low != "ep" && low != "playlist"
                })
                .collect();
            if let Some(a) = non_badge.get(0) {
                artist = a.to_string();
            }
            if let Some(y) = non_badge.get(1) {
                year = Some(y.to_string());
            }
        }

        let raw_cover = header_val["thumbnail"]["croppedSquareThumbnailRenderer"]["thumbnail"]["thumbnails"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|t| t["url"].as_str())
            .or_else(|| {
                header_val["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|t| t["url"].as_str())
            });
        cover_art_url = upscale_yt_art(raw_cover);

        description = header_val["description"]["runs"][0]["text"].as_str().map(|s| s.to_string());
    }

    // 2. Tracklist parsing
    let single_col = json["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array();
    let two_col = json["contents"]["twoColumnBrowseResultsRenderer"]["secondaryContents"]["sectionListRenderer"]["contents"].as_array();
    let direct_sec = json["contents"]["sectionListRenderer"]["contents"].as_array();
    let contents = single_col.or(two_col).or(direct_sec);
    if let Some(sections) = contents {
        for section in sections {
            let shelf_items = section["musicShelfRenderer"]["contents"].as_array()
                .or_else(|| section["musicPlaylistShelfRenderer"]["contents"].as_array());

            if let Some(items) = shelf_items {
                for item in items {
                    if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                        if let Some(mut track) = parse_search_track_renderer(renderer) {
                            if track.artist.trim().is_empty() && !artist.trim().is_empty() {
                                track.artist = artist.clone();
                            }
                            if track.album.is_none() && !title.trim().is_empty() {
                                track.album = Some(title.clone());
                            }
                            if track.cover_art_url.is_none() {
                                track.cover_art_url = cover_art_url.clone();
                            }
                            tracks.push(track);
                        }
                    }
                }
            }
        }
    }

    track_count = Some(tracks.len() as u32);

    let album_detail = AlbumDetailResult {
        id: browse_id,
        title,
        artist,
        year,
        description,
        cover_art_url,
        track_count,
        tracks,
    };

    Ok(serde_json::to_string(&album_detail)?)
}

#[plugin_fn]
pub fn browse_artist(input: String) -> FnResult<String> {
    let browse_id: String = serde_json::from_str(&input).unwrap_or(input);
    unsafe { host_log(format!("WASM browse_artist for: {}", browse_id))? };

    let vd = get_visitor_data().unwrap_or_default();
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20240101.01.00",
                "visitorData": vd
            }
        },
        "browseId": browse_id
    });

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());

    let res = do_http("POST", "https://music.youtube.com/youtubei/v1/browse", Some(headers), Some(body.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&res.body).unwrap_or_default();

    let mut name = String::new();
    let mut avatar_url = None;
    let mut banner_url = None;
    let mut subscribers = None;
    let mut bio = None;
    let mut top_tracks = Vec::new();
    let mut albums = Vec::new();
    let mut singles = Vec::new();
    let mut videos = Vec::new();
    let mut featured_on = Vec::new();
    let mut similar_artists = Vec::new();
    let mut seen_singles_keys = std::collections::HashSet::new();

    // 1. Header parsing
    let header_opt = json["header"]["musicImmersiveHeaderRenderer"].as_object()
        .or_else(|| json["header"]["musicVisualHeaderRenderer"].as_object());

    if let Some(header) = header_opt {
        let header_val = serde_json::Value::Object(header.clone());
        if let Some(n) = header_val["title"]["runs"][0]["text"].as_str() {
            name = n.to_string();
        }

        subscribers = header_val["subscriptionButton"]["subscribeButtonRenderer"]["subscriberCountText"]["runs"][0]["text"]
            .as_str().map(|s| s.to_string());

        bio = header_val["description"]["runs"][0]["text"].as_str().map(|s| s.to_string());

        let raw_avatar = header_val["foregroundThumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
            .as_array()
            .and_then(|arr| arr.last())
            .and_then(|t| t["url"].as_str())
            .or_else(|| {
                header_val["thumbnail"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                    .as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|t| t["url"].as_str())
            });
        avatar_url = upscale_yt_art(raw_avatar);
    }

    // 2. Shelves parsing
    let contents = json["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]["contents"].as_array();
    if let Some(sections) = contents {
        for section in sections {
            // A. Top Songs shelf
            if let Some(shelf) = section.get("musicShelfRenderer") {
                let title = shelf["title"]["runs"][0]["text"].as_str().unwrap_or("").to_lowercase();
                if title.contains("song") || title.contains("track") {
                    if let Some(items) = shelf["contents"].as_array() {
                        for item in items {
                            if let Some(renderer) = item.get("musicResponsiveListItemRenderer") {
                                if let Some(mut track) = parse_search_track_renderer(renderer) {
                                    if track.artist.trim().is_empty() && !name.trim().is_empty() {
                                        track.artist = name.clone();
                                    }
                                    if track.cover_art_url.is_none() {
                                        track.cover_art_url = avatar_url.clone();
                                    }
                                    top_tracks.push(track);
                                }
                            }
                        }
                    }
                }
            }
            // B. Albums, Singles, Videos, Featured On & Similar Artists Carousels
            else if let Some(carousel) = section.get("musicCarouselShelfRenderer") {
                let title = carousel["header"]["musicCarouselShelfBasicHeaderRenderer"]["title"]["runs"][0]["text"]
                    .as_str().unwrap_or("").to_lowercase();

                if let Some(items) = carousel["contents"].as_array() {
                    for item in items {
                        if let Some(renderer) = item.get("musicTwoRowItemRenderer") {
                            let item_title = renderer["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                            let nav = &renderer["navigationEndpoint"];
                            let item_browse_id = nav["browseEndpoint"]["browseId"].as_str().unwrap_or("").to_string();
                            let item_video_id = nav["watchEndpoint"]["videoId"].as_str().unwrap_or("").to_string();

                            let item_year = renderer["subtitle"]["runs"].as_array()
                                .and_then(|runs| runs.last())
                                .and_then(|r| r["text"].as_str())
                                .map(|s| s.to_string());
                            let item_cover = upscale_yt_art(
                                renderer["thumbnailRenderer"]["musicThumbnailRenderer"]["thumbnail"]["thumbnails"]
                                    .as_array()
                                    .and_then(|arr| arr.last())
                                    .and_then(|t| t["url"].as_str())
                            );

                            if title.contains("video") {
                                let vid_id = if !item_video_id.is_empty() { item_video_id } else { item_browse_id.clone() };
                                if !vid_id.is_empty() {
                                    videos.push(TrackResult {
                                        id: vid_id,
                                        title: item_title,
                                        artist: name.clone(),
                                        album: None,
                                        cover_art_url: item_cover,
                                        stream_url: None,
                                        quality_hint: None,
                                        duration_ms: None,
                                plays: None,
                                    });
                                }
                            } else if title.contains("featured") || title.contains("appears") {
                                if !item_browse_id.is_empty() {
                                    let author = renderer["subtitle"]["runs"][0]["text"].as_str().map(|s| s.to_string());
                                    featured_on.push(PlaylistItem {
                                        id: item_browse_id,
                                        title: item_title,
                                        author,
                                        item_count: None,
                                        cover_art_url: item_cover,
                                    });
                                }
                            } else if title.contains("fans") || title.contains("similar") || title.contains("artist") {
                                if !item_browse_id.is_empty() {
                                    let subs = renderer["subtitle"]["runs"][0]["text"].as_str().map(|s| s.to_string());
                                    similar_artists.push(ArtistItem {
                                        id: item_browse_id,
                                        name: item_title,
                                        avatar_url: item_cover,
                                        subscribers: subs,
                                    });
                                }
                            } else if title.contains("single") || title.contains("ep") {
                                if !item_browse_id.is_empty() {
                                    let key = (item_title.trim().to_lowercase(), item_year.clone().unwrap_or_default());
                                    if !seen_singles_keys.contains(&key) {
                                        seen_singles_keys.insert(key);
                                        singles.push(AlbumItem {
                                            id: item_browse_id,
                                            title: item_title,
                                            artist: name.clone(),
                                            year: item_year,
                                            cover_art_url: item_cover,
                                        });
                                    }
                                }
                            } else {
                                if !item_browse_id.is_empty() {
                                    albums.push(AlbumItem {
                                        id: item_browse_id,
                                        title: item_title,
                                        artist: name.clone(),
                                        year: item_year,
                                        cover_art_url: item_cover,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let artist_detail = ArtistDetailResult {
        id: browse_id,
        name,
        avatar_url,
        banner_url,
        subscribers,
        bio,
        top_tracks,
        albums,
        singles,
        videos,
        featured_on,
        similar_artists,
    };

    Ok(serde_json::to_string(&artist_detail)?)
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

fn fetch_innertube_next_related(video_id: &str) -> FnResult<Vec<TrackResult>> {
    let vd = get_visitor_data().unwrap_or_default();
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20250101.01.00",
                "visitorData": vd
            }
        },
        "playlistId": format!("RDAMVM{}", video_id),
        "videoId": video_id,
        "isAudioOnly": true,
        "enablePersistentPlaylistPanel": true
    });

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Referer".to_string(), "https://music.youtube.com/".to_string());

    let res = do_http("POST", "https://music.youtube.com/youtubei/v1/next", Some(headers), Some(body.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&res.body).unwrap_or_default();

    let mut tracks = Vec::new();

    if let Some(panel_contents) = json["contents"]["singleColumnMusicWatchNextResultsRenderer"]["tabbedRenderer"]["watchNextTabbedResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["musicQueueRenderer"]["content"]["playlistPanelRenderer"]["contents"].as_array() {
        for item in panel_contents {
            if let Some(renderer) = item.get("playlistPanelVideoRenderer") {
                let id = renderer["videoId"].as_str().unwrap_or("").to_string();
                if id.is_empty() || id == video_id {
                    continue;
                }
                let title = renderer["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                let mut artist = String::new();
                let mut album = None;
                let mut duration_ms = None;

                if let Some(sub_runs) = renderer["longBylineText"]["runs"].as_array().or_else(|| renderer["shortBylineText"]["runs"].as_array()) {
                    let parts: Vec<&str> = sub_runs.iter().filter_map(|r| r["text"].as_str()).filter(|t| *t != " • " && *t != " \u{2022} ").collect();
                    if let Some(a) = parts.get(0) {
                        artist = a.to_string();
                    }
                    if parts.len() >= 2 {
                        album = Some(parts[1].to_string());
                    }
                }

                if let Some(dur_text) = renderer["lengthText"]["runs"][0]["text"].as_str() {
                    duration_ms = parse_duration_ms(dur_text);
                }

                let music_video_type = renderer["navigationEndpoint"]["watchEndpoint"]["watchEndpointMusicSupportedConfigs"]["watchEndpointMusicConfig"]["musicVideoType"]
                    .as_str()
                    .or_else(|| renderer["menu"]["menuRenderer"]["items"][0]["menuNavigationItemRenderer"]["navigationEndpoint"]["watchEndpoint"]["watchEndpointMusicSupportedConfigs"]["watchEndpointMusicConfig"]["musicVideoType"].as_str());

                if !is_official_music_track(&title, &artist, music_video_type) {
                    continue;
                }

                if artist.ends_with(" - Topic") {
                    artist = artist.trim_end_matches(" - Topic").to_string();
                }

                let raw_cover = renderer["thumbnail"]["thumbnails"].as_array()
                    .and_then(|arr| arr.last())
                    .and_then(|t| t["url"].as_str());
                let cover_art_url = upscale_yt_art(raw_cover);

                tracks.push(TrackResult {
                    id,
                    title,
                    artist,
                    album,
                    cover_art_url,
                    stream_url: None,
                    quality_hint: None,
                    duration_ms,
                                plays: None,
                });
            }
        }
    }

    Ok(tracks)
}

fn clean_seed_artist_and_title(artist: &str, title: &str) -> (String, String) {
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

fn do_get_related(input: &str) -> FnResult<String> {
    let seed: CanonicalSeedV1 = serde_json::from_str(input)?;
    let (clean_artist, clean_title) = clean_seed_artist_and_title(&seed.artist, &seed.title);
    let _ = unsafe { host_log(format!("[RECOMMEND] Fetching artist catalog for seed: {} (clean: {} - {})", seed.artist, clean_artist, clean_title)) };

    // Query artist songs for rich, varied catalog & vibe recommendations
    let query = if !clean_artist.is_empty() {
        format!("{} songs", clean_artist)
    } else {
        clean_title.clone()
    };

    let search_input = serde_json::json!({
        "query": query,
        "filter": "songs"
    }).to_string();

    if let Ok(categorized) = do_search_categorized(search_input) {
        if let Ok(parsed) = serde_json::from_str::<CategorizedSearchResult>(&categorized) {
            let mut tracks = Vec::new();
            let lower_seed_title = clean_title.to_lowercase();
            let lower_seed_artist = clean_artist.to_lowercase();

            for sec in parsed.sections {
                for item in sec.items {
                    if let SearchItem::Track(t) = item {
                        let t_title_lower = t.title.to_lowercase();
                        let t_artist_lower = t.artist.to_lowercase();

                        // Filter out non-song noise, mashups, binaural meditation loops
                        let noise_keywords = [
                            "mashup", "binaural", "meditation", "432hz", "528hz", 
                            "1 hour", "10 hours", "relaxing background", "sleep music", "iq boost"
                        ];
                        if noise_keywords.iter().any(|k| t_title_lower.contains(k)) {
                            continue;
                        }

                        // Robust seed song exclusion: Exclude any variation of the seed track by the seed artist
                        let is_artist_match = lower_seed_artist.is_empty() 
                            || t_artist_lower.contains(&lower_seed_artist) 
                            || lower_seed_artist.contains(&t_artist_lower);

                        let is_title_match = !lower_seed_title.is_empty() && (
                            t_title_lower == lower_seed_title 
                            || t_title_lower.contains(&lower_seed_title) 
                            || lower_seed_title.contains(&t_title_lower)
                        );

                        if is_artist_match && is_title_match {
                            continue; // Skip duplicate upload/cover/remix of the seed song itself
                        }

                        tracks.push(t);
                    }
                }
            }

            if !tracks.is_empty() {
                return Ok(serde_json::to_string(&tracks)?);
            }
        }
    }

    let empty: Vec<TrackResult> = Vec::new();
    Ok(serde_json::to_string(&empty)?)
}

#[plugin_fn]
pub fn get_related(input: String) -> FnResult<String> {
    do_get_related(&input)
}

fn do_get_radio(input: &str) -> FnResult<String> {
    let seed: CanonicalSeedV1 = serde_json::from_str(input)?;
    let _ = unsafe { host_log(format!("[RADIO] Generating Automix radio for seed: {} - {}", seed.artist, seed.title)) };

    // 1. If seed has native_id (videoId), directly fetch Automix radio
    if let Some(vid) = &seed.native_id {
        if !vid.is_empty() {
            if let Ok(tracks) = fetch_innertube_next_related(vid) {
                if !tracks.is_empty() {
                    let result = RadioStreamResultV1 {
                        tracks,
                        continuation_token: None,
                    };
                    return Ok(serde_json::to_string(&result)?);
                }
            }
        }
    }

    // 2. Resolve videoId by searching artist + title
    let (clean_artist, clean_title) = clean_seed_artist_and_title(&seed.artist, &seed.title);
    let search_query = if !clean_artist.is_empty() && !clean_title.is_empty() {
        format!("{} {}", clean_artist, clean_title)
    } else if !clean_artist.is_empty() {
        format!("{} songs", clean_artist)
    } else {
        clean_title
    };

    let search_input = serde_json::json!({
        "query": search_query,
        "filter": "songs"
    }).to_string();

    if let Ok(categorized) = do_search_categorized(search_input) {
        if let Ok(parsed) = serde_json::from_str::<CategorizedSearchResult>(&categorized) {
            for sec in parsed.sections {
                for item in sec.items {
                    if let SearchItem::Track(t) = item {
                        if !t.id.is_empty() {
                            if let Ok(automix_tracks) = fetch_innertube_next_related(&t.id) {
                                if !automix_tracks.is_empty() {
                                    let result = RadioStreamResultV1 {
                                        tracks: automix_tracks,
                                        continuation_token: None,
                                    };
                                    return Ok(serde_json::to_string(&result)?);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback to related search if Automix is empty
    let related_json = do_get_related(input)?;
    let tracks: Vec<TrackResult> = serde_json::from_str(&related_json).unwrap_or_default();
    let result = RadioStreamResultV1 {
        tracks,
        continuation_token: None,
    };
    Ok(serde_json::to_string(&result)?)
}

#[plugin_fn]
pub fn get_radio(input: String) -> FnResult<String> {
    do_get_radio(&input)
}

#[plugin_fn]
pub fn on_playback_event(input: String) -> FnResult<String> {
    let event: PlaybackTelemetryEventV1 = match serde_json::from_str(&input) {
        Ok(e) => e,
        Err(_) => return Ok("ignored".to_string()),
    };

    let _ = unsafe { host_log(format!("[TELEMETRY] Playback event for track {}: {}ms / {}ms (completed: {})", event.native_track_id, event.duration_ms, event.total_track_duration_ms, event.completed)) };

    let req = HttpRequest {
        method: "POST".to_string(),
        url: "https://music.youtube.com/youtubei/v1/feedback".to_string(),
        headers: None,
        body: Some(serde_json::json!({
            "feedbackTokens": [event.native_track_id],
            "isAggregated": true
        }).to_string()),
    };
    if let Ok(json_req) = serde_json::to_string(&req) {
        let _ = unsafe { host_telemetry_request(json_req) };
    }

    Ok("ok".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_player_hash() {
        // Escaped iframe_api response
        let iframe_body = r#"yt.setConfig({'PLAYER_JS_URL': '\/s\/player\/d253cfc5\/player_ias.vflset\/en_US\/base.js'});"#;
        assert_eq!(extract_player_hash(iframe_body), Some("d253cfc5".to_string()));

        // Direct web URL format
        let web_body = r#"<script src="/s/player/9c249f6f/player_ias.vflset/en_GB/base.js"></script>"#;
        assert_eq!(extract_player_hash(web_body), Some("9c249f6f".to_string()));

        // Negative case
        assert_eq!(extract_player_hash("random html body without player"), None);
    }

    #[test]
    fn test_extract_signature_timestamp() {
        // Anchored signatureTimestamp pattern
        let js_anchored = r#"var a = {signatureTimestamp:20123, other: 1};"#;
        assert_eq!(extract_signature_timestamp(js_anchored), Some(20123));

        // Anchored with quotes and spacing
        let js_quoted = r#"{"signatureTimestamp": 20125, "sts": 20111}"#;
        assert_eq!(extract_signature_timestamp(js_quoted), Some(20125));

        // Loose sts fallback pattern
        let js_loose = r#"var b = {sts: 20119, foo: 'bar'};"#;
        assert_eq!(extract_signature_timestamp(js_loose), Some(20119));

        // Negative / malformed
        assert_eq!(extract_signature_timestamp("var sts = 0;"), None);
        assert_eq!(extract_signature_timestamp("no timestamp here"), None);
    }



    #[test]
    fn test_embedded_client_body() {
        let client = YtClient {
            name: "TVHTML5_SIMPLY_EMBEDDED_PLAYER", version: "2.0", client_id: "85",
            user_agent: "test_ua", needs_potoken: false, needs_sts: true,
            include_ua_in_context: false, is_embedded: true,
            os_name: None, os_version: None, device_make: None, device_model: None,
            android_sdk_version: None,
        };
        let body = build_player_body(&client, "dQw4w9WgXcQ", "test_vd", None, Some(20125));
        assert_eq!(body["context"]["thirdParty"]["embedUrl"], "https://www.reddit.com/");
        assert_eq!(body["playbackContext"]["contentPlaybackContext"]["signatureTimestamp"], 20125);
    }

    #[test]
    fn test_parse_signature_cipher() {
        let raw = "s=test_sig_1234567890&sp=sig&url=https%3A%2F%2Frr1---sn.googlevideo.com%2Fvideoplayback%3Fexpire%3D123";
        let parsed = parse_signature_cipher(raw);
        assert!(parsed.is_some());
        let (s, sp, url) = parsed.unwrap();
        assert_eq!(s, "test_sig_1234567890");
        assert_eq!(sp, "sig");
        assert_eq!(url, "https://rr1---sn.googlevideo.com/videoplayback?expire=123");
    }

    #[test]
    fn test_url_decode_component() {
        assert_eq!(url_decode_component("https%3A%2F%2Fexample.com%2Fpath%3Ffoo%3Dbar%20baz"), "https://example.com/path?foo=bar baz");
    }


    #[test]
    fn test_parse_charts_tracks() {
        let json = serde_json::json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "itemSectionRenderer": {
                                            "contents": [{
                                                "musicShelfRenderer": {
                                                    "contents": [{
                                                        "musicResponsiveListItemRenderer": {
                                                            "playlistItemData": { "videoId": "chart_vid_1" },
                                                            "flexColumns": [
                                                                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [{ "text": "Chart Song 1" }] } } },
                                                                { "musicResponsiveListItemFlexColumnRenderer": { "text": { "runs": [{ "text": "Chart Artist 1" }] } } }
                                                            ],
                                                            "thumbnail": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [{ "url": "https://img.youtube.com/vi/1.jpg" }] } } }
                                                        }
                                                    }]
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let items = parse_charts_tracks(&json, None);
        assert_eq!(items.len(), 1);
        match &items[0] {
            ModuleItem::Track(t) => {
                assert_eq!(t.id, "chart_vid_1");
                assert_eq!(t.title, "Chart Song 1");
                assert_eq!(t.artist, "Chart Artist 1");
                assert_eq!(t.quality_hint, Some("#1".to_string()));
            }
            _ => panic!("Expected Track"),
        }
    }

    #[test]
    fn test_parse_new_releases_albums() {
        let json = serde_json::json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "musicCarouselShelfRenderer": {
                                            "contents": [{
                                                "musicTwoRowItemRenderer": {
                                                    "navigationEndpoint": { "browseEndpoint": { "browseId": "MPREb_album1" } },
                                                    "title": { "runs": [{ "text": "Album Title" }] },
                                                    "subtitle": { "runs": [{ "text": "Album Artist" }, { "text": "2026" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [{ "url": "https://example.com/cover.jpg" }] } } }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let items = parse_new_releases_albums(&json);
        assert_eq!(items.len(), 1);
        match &items[0] {
            ModuleItem::Album(a) => {
                assert_eq!(a.id, "MPREb_album1");
                assert_eq!(a.title, "Album Title");
                assert_eq!(a.artist, "Album Artist");
            }
            _ => panic!("Expected Album"),
        }
    }

    #[test]
    fn test_extract_n_param_from_url() {
        // n at beginning of query
        let url1 = "https://rr1---sn-4g5ednle.googlevideo.com/videoplayback?n=abc123XYZ&itag=251";
        assert_eq!(extract_n_param_from_url(url1), Some("abc123XYZ".to_string()));

        // n in middle of query
        let url2 = "https://rr1---sn-4g5ednle.googlevideo.com/videoplayback?expire=123&n=KdrqFlzJXl9EcCwlmEy&sparams=expire";
        assert_eq!(extract_n_param_from_url(url2), Some("KdrqFlzJXl9EcCwlmEy".to_string()));

        // n missing
        let url3 = "https://rr1---sn-4g5ednle.googlevideo.com/videoplayback?expire=123&itag=251";
        assert_eq!(extract_n_param_from_url(url3), None);
    }

    #[test]
    fn test_extract_youtube_video_id() {
        assert_eq!(extract_youtube_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"), Some("dQw4w9WgXcQ".to_string()));
        assert_eq!(extract_youtube_video_id("https://music.youtube.com/watch?v=dQw4w9WgXcQ&feature=share"), Some("dQw4w9WgXcQ".to_string()));
        assert_eq!(extract_youtube_video_id("https://youtu.be/dQw4w9WgXcQ"), Some("dQw4w9WgXcQ".to_string()));
        assert_eq!(extract_youtube_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"), Some("dQw4w9WgXcQ".to_string()));
        assert_eq!(extract_youtube_video_id("https://soundcloud.com/artist/track"), None);
        assert_eq!(extract_youtube_video_id("hello world"), None);
    }

    #[test]
    fn test_sanitize_module_items() {
        let items = vec![
            ModuleItem::Album(AlbumItem {
                id: "MPREb_valid".into(),
                title: "Random Access Memories".into(),
                artist: "Daft Punk".into(),
                year: Some("2013".into()),
                cover_art_url: None,
            }),
            ModuleItem::Album(AlbumItem {
                id: "MPREb_junk1".into(),
                title: "Starboy (Slowed + Reverb)".into(),
                artist: "DJ Spammer".into(),
                year: None,
                cover_art_url: None,
            }),
            ModuleItem::Album(AlbumItem {
                id: "MPREb_junk2".into(),
                title: "Cool Ringtone Soundboard".into(),
                artist: "Ringtone Master".into(),
                year: None,
                cover_art_url: None,
            }),
            ModuleItem::Track(TrackResult {
                id: "dQw4w9WgXcQ".into(),
                title: "Never Gonna Give You Up [Official Music Video]".into(),
                artist: "Rick Astley".into(),
                album: None,
                cover_art_url: None,
                stream_url: None,
                quality_hint: None,
                duration_ms: Some(213000),
                                plays: None,
            }),
        ];

        let sanitized = sanitize_module_items(items);
        assert_eq!(sanitized.len(), 2);
        match &sanitized[0] {
            ModuleItem::Album(a) => {
                assert_eq!(a.id, "MPREb_valid");
                assert_eq!(a.title, "Random Access Memories");
            }
            _ => panic!("Expected valid album"),
        }
        match &sanitized[1] {
            ModuleItem::Track(t) => {
                assert_eq!(t.id, "dQw4w9WgXcQ");
                assert_eq!(t.title, "Never Gonna Give You Up");
            }
            _ => panic!("Expected clean track"),
        }
    }
}


fn get_unix_timestamp() -> u64 {
    // Safe timestamp for wasm32-unknown-unknown target without OS clock syscalls
    1700000000
}
