use extism_pdk::*;
use std::collections::HashMap;
use crate::models::*;
use crate::host::{do_http, host_log};
use crate::botguard::get_visitor_data;

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

pub fn parse_charts_tracks(json: &serde_json::Value, target_shelf: Option<&str>) -> Vec<ModuleItem> {
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

pub fn parse_new_releases_albums(json: &serde_json::Value) -> Vec<ModuleItem> {
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


pub fn search_impl(input: String) -> FnResult<String> {
    do_search(input)
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


pub fn search_categorized_impl(input: String) -> FnResult<String> {
    do_search_categorized(input)
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


pub fn get_modules_impl() -> FnResult<String> {
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


pub fn fetch_module_impl(input: String) -> FnResult<String> {
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


pub fn browse_album_impl(input: String) -> FnResult<String> {
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


pub fn browse_artist_impl(input: String) -> FnResult<String> {
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


pub fn fetch_innertube_next_related(video_id: &str) -> FnResult<Vec<TrackResult>> {
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


pub fn get_related_impl(input: &str) -> FnResult<String> {
    do_get_related(input)
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


pub fn get_radio_impl(input: &str) -> FnResult<String> {
    do_get_radio(input)
}

