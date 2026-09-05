use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::botguard::USER_AGENT;
use crate::host::{
    do_http, get_storage, set_storage, get_unix_timestamp,
    url_decode_component, host_log, host_execute_webview_js,
};

const STS_CACHE_TTL_SECS: u64 = 6 * 60 * 60; // 6 hours

#[derive(Serialize, Deserialize)]
struct StsCache {
    sts: i64,
    player_hash: String,
    timestamp_secs: u64,
}

const ZEMER_CONFIG_URL: &str = "https://raw.githubusercontent.com/ZemerTeam/zemer-cipher/master/library/src/main/assets/player_configs.json";
const ZEMER_CONFIG_TTL_SECS: u64 = 6 * 60 * 60; // 6 hours, matching Metrolist

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZemerPlayerEntry {
    pub sig: String,
    #[serde(rename = "nClass")]
    pub n_class: String,
    pub sts: i64,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZemerConfigs {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub players: HashMap<String, ZemerPlayerEntry>,
}

pub fn get_embedded_player_configs() -> HashMap<String, ZemerPlayerEntry> {
    let mut map = HashMap::new();
    
    // Recent validated configs from Zemer/Metrolist player_configs.json
    map.insert("f572e43c".to_string(), ZemerPlayerEntry {
        sig: "rY(18,3016,INPUT)".to_string(),
        n_class: "um".to_string(),
        sts: 20697,
        aliases: vec!["8d57721e".to_string()],
    });
    map.insert("9470c977".to_string(), ZemerPlayerEntry {
        sig: "Of(2,137,INPUT)".to_string(),
        n_class: "wO".to_string(),
        sts: 20696,
        aliases: vec!["9f1ba9db".to_string()],
    });
    map.insert("4a9ed7b5".to_string(), ZemerPlayerEntry {
        sig: "WQ(14,8942,INPUT)".to_string(),
        n_class: "ty".to_string(),
        sts: 20698,
        aliases: vec!["cc2d64e5".to_string()],
    });
    map.insert("95bf8a44".to_string(), ZemerPlayerEntry {
        sig: "WQ(14,8942,INPUT)".to_string(),
        n_class: "ty".to_string(),
        sts: 20698,
        aliases: vec!["975af835".to_string()],
    });
    map.insert("fc590a67".to_string(), ZemerPlayerEntry {
        sig: "WQ(14,8942,INPUT)".to_string(),
        n_class: "ty".to_string(),
        sts: 20698,
        aliases: vec!["c859a220".to_string()],
    });
    map.insert("1ceea3e8".to_string(), ZemerPlayerEntry {
        sig: "WQ(14,8942,INPUT)".to_string(),
        n_class: "ty".to_string(),
        sts: 20698,
        aliases: vec!["319fe431".to_string()],
    });
    map.insert("ef64e108".to_string(), ZemerPlayerEntry {
        sig: "WQ(14,8942,INPUT)".to_string(),
        n_class: "ty".to_string(),
        sts: 20698,
        aliases: vec!["05ad8578".to_string()],
    });
    map.insert("ce167ec0".to_string(), ZemerPlayerEntry {
        sig: "dS(7,2166,INPUT)".to_string(),
        n_class: "np".to_string(),
        sts: 20699,
        aliases: vec!["f8b6ed41".to_string()],
    });
    map.insert("259c7f44".to_string(), ZemerPlayerEntry {
        sig: "WQ(14,8942,INPUT)".to_string(),
        n_class: "ty".to_string(),
        sts: 20698,
        aliases: vec!["c01ca9bc".to_string()],
    });
    map.insert("10733253".to_string(), ZemerPlayerEntry {
        sig: "dS(7,2166,INPUT)".to_string(),
        n_class: "np".to_string(),
        sts: 20699,
        aliases: vec!["27a3833a".to_string()],
    });
    map.insert("3e7a0d91".to_string(), ZemerPlayerEntry {
        sig: "mp(7,2166,INPUT)".to_string(),
        n_class: "Ou".to_string(),
        sts: 20699,
        aliases: vec!["91e673b7".to_string()],
    });
    map.insert("a52cf1ce".to_string(), ZemerPlayerEntry {
        sig: "mp(7,2166,INPUT)".to_string(),
        n_class: "Ou".to_string(),
        sts: 20699,
        aliases: vec!["3f877f13".to_string()],
    });
    map.insert("38e8e189".to_string(), ZemerPlayerEntry {
        sig: "jg(7,2166,INPUT)".to_string(),
        n_class: "FB".to_string(),
        sts: 20699,
        aliases: vec!["d88e2876".to_string()],
    });
    map.insert("d109b9c2".to_string(), ZemerPlayerEntry {
        sig: "jg(7,2166,INPUT)".to_string(),
        n_class: "FB".to_string(),
        sts: 20699,
        aliases: vec!["f2db51af".to_string()],
    });
    map.insert("ab18ea88".to_string(), ZemerPlayerEntry {
        sig: "Df(8,6565,INPUT)".to_string(),
        n_class: "jm".to_string(),
        sts: 20700,
        aliases: vec!["1ba7d311".to_string()],
    });
    map.insert("1b29db7a".to_string(), ZemerPlayerEntry {
        sig: "Df(8,6565,INPUT)".to_string(),
        n_class: "jm".to_string(),
        sts: 20700,
        aliases: vec!["a41d7c8a".to_string()],
    });
    map.insert("b93b66db".to_string(), ZemerPlayerEntry {
        sig: "QV(8,6565,INPUT)".to_string(),
        n_class: "M7".to_string(),
        sts: 20700,
        aliases: vec!["488ab3ed".to_string()],
    });
    map.insert("e2a2364f".to_string(), ZemerPlayerEntry {
        sig: "dE(8,6565,INPUT)".to_string(),
        n_class: "sI".to_string(),
        sts: 20700,
        aliases: vec!["9bec666d".to_string()],
    });
    map.insert("9c249f6f".to_string(), ZemerPlayerEntry {
        sig: "Tl(48,5831,INPUT)".to_string(),
        n_class: "W_".to_string(),
        sts: 20602,
        aliases: vec!["a6fc27c5".to_string()],
    });
    map.insert("4f38b487".to_string(), ZemerPlayerEntry {
        sig: "Tl(48,5831,INPUT)".to_string(),
        n_class: "W_".to_string(),
        sts: 20602,
        aliases: vec!["1215646b".to_string()],
    });
    map.insert("1d81eac9".to_string(), ZemerPlayerEntry {
        sig: "tq(1,8309,INPUT)".to_string(),
        n_class: "Y_".to_string(),
        sts: 20691,
        aliases: vec!["a25c54c3".to_string()],
    });
    
    map
}

pub fn fetch_and_cache_zemer_configs(force: bool) -> Option<ZemerConfigs> {
    let now = get_unix_timestamp();
    if !force {
        if let Ok(cached_json) = get_storage("zemer_player_configs") {
            if !cached_json.is_empty() {
                let cached_time: u64 = get_storage("zemer_player_configs_time")
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(0);
                if now >= cached_time && (now - cached_time) < ZEMER_CONFIG_TTL_SECS {
                    if let Ok(configs) = serde_json::from_str::<ZemerConfigs>(&cached_json) {
                        return Some(configs);
                    }
                }
            }
        }
    }

    let _ = unsafe { host_log("[CIPHER] Fetching remote player configs from Zemer repository...".to_string()) };
    let mut headers = HashMap::new();
    headers.insert("User-Agent".to_string(), USER_AGENT.to_string());
    if let Ok(resp) = do_http("GET", ZEMER_CONFIG_URL, Some(headers), None) {
        if resp.status == 200 {
            if let Ok(configs) = serde_json::from_str::<ZemerConfigs>(&resp.body) {
                let _ = set_storage("zemer_player_configs", &resp.body);
                let _ = set_storage("zemer_player_configs_time", &now.to_string());
                let _ = unsafe { host_log(format!("[CIPHER] Loaded {} player configs from remote", configs.players.len())) };
                return Some(configs);
            }
        }
    }

    if let Ok(cached_json) = get_storage("zemer_player_configs") {
        if let Ok(configs) = serde_json::from_str::<ZemerConfigs>(&cached_json) {
            let _ = unsafe { host_log("[CIPHER] Falling back to stale cached player configs".to_string()) };
            return Some(configs);
        }
    }

    None
}

pub fn get_player_config(player_hash: &str) -> Option<ZemerPlayerEntry> {
    // 1. Try remote/cached Zemer configs (without force)
    if let Some(configs) = fetch_and_cache_zemer_configs(false) {
        if let Some(entry) = configs.players.get(player_hash) {
            return Some(entry.clone());
        }
        for (_, entry) in &configs.players {
            if entry.aliases.iter().any(|a| a == player_hash) {
                return Some(entry.clone());
            }
        }
    }

    // 2. Try embedded configs before forcing network refresh
    let embedded = get_embedded_player_configs();
    if let Some(entry) = embedded.get(player_hash) {
        return Some(entry.clone());
    }
    for (_, entry) in &embedded {
        if entry.aliases.iter().any(|a| a == player_hash) {
            return Some(entry.clone());
        }
    }

    // 3. Force refresh remote configs (self-heal for new/rotated player hash)
    let _ = unsafe { host_log(format!("[CIPHER] Player hash {} not found locally, forcing remote config refresh", player_hash)) };
    if let Some(configs) = fetch_and_cache_zemer_configs(true) {
        if let Some(entry) = configs.players.get(player_hash) {
            return Some(entry.clone());
        }
        for (_, entry) in &configs.players {
            if entry.aliases.iter().any(|a| a == player_hash) {
                return Some(entry.clone());
            }
        }
    }

    None
}

pub fn extract_player_hash(content: &str) -> Option<String> {
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

pub fn extract_signature_timestamp(player_js: &str) -> Option<i64> {
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

pub fn fetch_player_hash_from_network() -> Option<String> {
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

pub fn fetch_player_js_and_sts(hash: &str) -> Option<(i64, String)> {
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
            if let Some(cfg) = get_player_config(hash) {
                if cfg.sts > 10000 {
                    let _ = unsafe { host_log(format!("[STS] Using signatureTimestamp {} from validated config for {}", cfg.sts, hash)) };
                    return Some((cfg.sts, resp.body));
                }
            }
        }
    }
    None
}


pub fn parse_signature_cipher(cipher_str: &str) -> Option<(String, String, String)> {
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

pub fn deobfuscate_signature(obfuscated_s: &str, player_hash: &str) -> String {
    if !init_sandbox_player_js_if_needed(player_hash) {
        let _ = unsafe { host_log("[CIPHER] Sandbox not initialized, returning raw signature".to_string()) };
        return obfuscated_s.to_string();
    }

    let escaped_s = obfuscated_s.replace('\\', "\\\\").replace('\'', "\\'");
    let decipher_script = format!(
        r#"(function() {{
            try {{
                var fn = window.__yt_sig_decipher || window._cipherSigFunc;
                if (typeof fn === 'function') {{
                    var res = fn('{escaped_s}');
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
            if !trimmed.is_empty() && trimmed.len() >= 10 && trimmed != obfuscated_s {
                let _ = unsafe { host_log(format!("[CIPHER] Successfully deciphered signature: {} chars -> {} chars", obfuscated_s.len(), trimmed.len())) };
                trimmed
            } else {
                let _ = unsafe { host_log(format!("[CIPHER] Decipher returned raw or invalid result (len: {})", trimmed.len())) };
                obfuscated_s.to_string()
            }
        }
        Err(e) => {
            let _ = unsafe { host_log(format!("[CIPHER] Failed to execute decipher JS: {:?}", e)) };
            obfuscated_s.to_string()
        }
    }
}

pub fn extract_n_param_from_url(url: &str) -> Option<String> {
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

pub fn init_sandbox_player_js_if_needed(player_hash: &str) -> bool {
    let check_js = format!(
        r#"typeof window.__yt_sig_decipher === 'function' && typeof window.__yt_n_transform === 'function' && window.__yt_loaded_player_hash === '{}';"#,
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
        let _ = unsafe { host_log("[CIPHER] Could not retrieve player.js for sandbox init".to_string()) };
        return false;
    }

    let config = match get_player_config(player_hash) {
        Some(c) => c,
        None => {
            let _ = unsafe { host_log(format!("[CIPHER] No validated cipher configuration found for player hash: {}", player_hash)) };
            return false;
        }
    };

    let sig_expr = config.sig.replace("INPUT", "sig");
    let n_class = &config.n_class;

    let export_code = format!(
        r#"; window.__yt_sig_decipher = window._cipherSigFunc = function(sig) {{
            try {{
                return {sig_expr};
            }} catch(e) {{
                return null;
            }}
        }};
        window.__yt_n_transform = window._nTransformFunc = function(n) {{
            try {{
                var u = new g.{n_class}('https://x.googlevideo.com/videoplayback?n=' + n, true);
                var t = u.get('n');
                return (t && t !== n) ? t : n;
            }} catch(e) {{
                return n;
            }}
        }};
        window.__yt_loaded_player_hash = '{player_hash}';"#
    );

    // Injection point: insert export statements before closure closing '})(_yt_player);'
    // Following Metrolist CipherWebView.kt implementation exactly
    let modified_js = if player_js.contains("})(_yt_player);") {
        player_js.replace("})(_yt_player);", &format!("{} }})(_yt_player);", export_code))
    } else {
        let _ = unsafe { host_log("[CIPHER] Closing closure marker '})(_yt_player);' not found, appending exports".to_string()) };
        format!("{}
{}", player_js, export_code)
    };

    let _ = unsafe { host_log(format!("[CIPHER] Initializing player.js (modified length: {} chars) with Zemer closure exports in JS Sandbox...", modified_js.len())) };

    match unsafe { host_execute_webview_js(modified_js) } {
        Ok(_) => {
            let verify_js = format!(
                r#"typeof window.__yt_sig_decipher === 'function' && typeof window.__yt_n_transform === 'function' && window.__yt_loaded_player_hash === '{}';"#,
                player_hash
            );
            if let Ok(res) = unsafe { host_execute_webview_js(verify_js) } {
                if res.trim() == "true" {
                    let _ = unsafe { host_log(format!("[CIPHER] Sandbox verified and ready for player hash {}", player_hash)) };
                    return true;
                }
            }
            let _ = unsafe { host_log("[CIPHER] Sandbox verification check failed after eval".to_string()) };
            false
        }
        Err(e) => {
            let _ = unsafe { host_log(format!("[CIPHER] Sandbox evaluation error: {:?}", e)) };
            false
        }
    }
}

pub fn transform_n_param(raw_n: &str, player_hash: &str) -> String {
    if !init_sandbox_player_js_if_needed(player_hash) {
        let _ = unsafe { host_log("[N-TRANSFORM] Sandbox not initialized, returning raw n".to_string()) };
        return raw_n.to_string();
    }

    let escaped_n = raw_n.replace('\\', "\\\\").replace('\'', "\\'");
    let transform_script = format!(
        r#"(function() {{
            try {{
                var fn = window.__yt_n_transform || window._nTransformFunc;
                if (typeof fn === 'function') {{
                    var res = fn('{escaped_n}');
                    if (typeof res === 'string' && res.length >= 5 && res !== '{escaped_n}') {{
                        return res;
                    }}
                }}
            }} catch(e) {{}}
            return '{escaped_n}';
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

pub fn apply_n_transform_to_url(url: &str, player_hash: &str) -> String {
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

pub fn get_or_refresh_sts() -> i64 {
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



