pub mod models;
pub mod host;
pub mod botguard;
pub mod cipher;
pub mod innertube;
pub mod explore;

use extism_pdk::*;
use models::*;
use host::{host_log, host_telemetry_request};

#[plugin_fn]
pub fn resolve(input: String) -> FnResult<String> {
    innertube::resolve_impl(input)
}

#[plugin_fn]
pub fn search(input: String) -> FnResult<String> {
    explore::search_impl(input)
}

#[plugin_fn]
pub fn search_categorized(input: String) -> FnResult<String> {
    explore::search_categorized_impl(input)
}

#[plugin_fn]
pub fn warmup(input: String) -> FnResult<String> {
    innertube::warmup_impl(input)
}

#[plugin_fn]
pub fn get_modules() -> FnResult<String> {
    explore::get_modules_impl()
}

#[plugin_fn]
pub fn fetch_module(input: String) -> FnResult<String> {
    explore::fetch_module_impl(input)
}

#[plugin_fn]
pub fn resolve_url(input: String) -> FnResult<String> {
    innertube::resolve_url_impl(input)
}

#[plugin_fn]
pub fn browse_album(input: String) -> FnResult<String> {
    explore::browse_album_impl(input)
}

#[plugin_fn]
pub fn browse_artist(input: String) -> FnResult<String> {
    explore::browse_artist_impl(input)
}

#[plugin_fn]
pub fn get_related(input: String) -> FnResult<String> {
    explore::get_related_impl(&input)
}

#[plugin_fn]
pub fn get_radio(input: String) -> FnResult<String> {
    explore::get_radio_impl(&input)
}

#[plugin_fn]
pub fn on_playback_event(input: String) -> FnResult<String> {
    let event: PlaybackTelemetryEventV1 = match serde_json::from_str(&input) {
        Ok(e) => e,
        Err(_) => return Ok("ignored".to_string()),
    };

    let _ = unsafe { host_log(format!("[TELEMETRY] Playback event for track {}: {}ms / {}ms (completed: {})", event.native_track_id, event.duration_ms, event.total_track_duration_ms, event.completed)) };

    let req = models::HttpRequest {
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
    use models::*;
    use host::*;
    use cipher::*;
    use innertube::*;
    use explore::*;

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

    #[test]
    fn test_embedded_player_configs() {
        let configs = get_embedded_player_configs();
        assert!(configs.contains_key("f572e43c"));
        let entry = &configs["f572e43c"];
        assert_eq!(entry.sig, "rY(18,3016,INPUT)");
        assert_eq!(entry.n_class, "um");
        assert_eq!(entry.sts, 20697);
        assert!(entry.aliases.contains(&"8d57721e".to_string()));

        // Test alias resolution via get_player_config
        let resolved = get_player_config("8d57721e");
        assert!(resolved.is_some());
        let res_entry = resolved.unwrap();
        assert_eq!(res_entry.sig, "rY(18,3016,INPUT)");
        assert_eq!(res_entry.n_class, "um");
    }

    #[test]
    fn test_zemer_player_config_parsing() {
        let sample_json = r#"{
            "schemaVersion": 1,
            "players": {
                "f572e43c": { "sig": "rY(18,3016,INPUT)", "nClass": "um", "sts": 20697, "aliases": ["8d57721e"] },
                "9470c977": { "sig": "Of(2,137,INPUT)", "nClass": "wO", "sts": 20696, "aliases": ["9f1ba9db"] }
            }
        }"#;

        let parsed = serde_json::from_str::<ZemerConfigs>(sample_json);
        assert!(parsed.is_ok());
        let configs = parsed.unwrap();
        assert_eq!(configs.schema_version, 1);
        assert_eq!(configs.players.len(), 2);
        assert_eq!(configs.players["f572e43c"].n_class, "um");
        assert_eq!(configs.players["f572e43c"].sig, "rY(18,3016,INPUT)");
    }

    #[test]
    fn test_closure_injection_into_player_js() {
        let fake_player = "var _yt_player={};(function(g){var window=this;g.um=function(url,b){return{get:function(k){return'transformed_n';}};};})(_yt_player);";
        let config = ZemerPlayerEntry {
            sig: "rY(18,3016,INPUT)".to_string(),
            n_class: "um".to_string(),
            sts: 20697,
            aliases: vec![],
        };
        let sig_expr = config.sig.replace("INPUT", "sig");
        let export_code = format!(
            "; window.__yt_sig_decipher = function(sig) {{ return {}; }}; window.__yt_n_transform = function(n) {{ var u = new g.{}(n); return u.get('n'); }};",
            sig_expr, config.n_class
        );
        let modified = fake_player.replace("})(_yt_player);", &format!("{} }})(_yt_player);", export_code));
        assert!(modified.contains("window.__yt_sig_decipher = function(sig) { return rY(18,3016,sig); };"));
        assert!(modified.contains("window.__yt_n_transform = function(n) { var u = new g.um(n); return u.get('n'); };"));
        assert!(modified.ends_with("})(_yt_player);"));
    }
}

