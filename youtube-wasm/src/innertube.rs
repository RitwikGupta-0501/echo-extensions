use extism_pdk::*;
use std::collections::HashMap;
use crate::models::{TrackResult, ResolvedTrack};
use crate::host::{do_http, get_storage, set_storage, url_encode_component, host_log};
use crate::botguard::{init_botguard, get_visitor_data, generate_po_token};
use crate::cipher::{
    get_or_refresh_sts, init_sandbox_player_js_if_needed,
    parse_signature_cipher, deobfuscate_signature, apply_n_transform_to_url,
};

pub struct YtClient {
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


pub fn fallback_clients() -> Vec<YtClient> {
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


pub fn build_player_body(client: &YtClient, track_id: &str, vd: &str, po_token: Option<&str>, sts: Option<i64>) -> serde_json::Value {
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


pub fn request_player(client: &YtClient, body: &serde_json::Value, vd: &str) -> FnResult<serde_json::Value> {
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


pub fn extract_youtube_video_id(url: &str) -> Option<String> {
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


pub fn resolve_impl(input: String) -> FnResult<String> {
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


pub fn warmup_impl(_input: String) -> FnResult<String> {
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


pub fn resolve_url_impl(input: String) -> FnResult<String> {
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


