use extism_pdk::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};

#[host_fn]
extern "ExtismHost" {
    fn host_execute_webview_js(script: String) -> String;
    fn host_log(msg: String);
    fn host_http_request(req: String) -> String;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub cover_art_url: Option<String>,
    pub stream_url: Option<String>,
    pub quality_hint: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResolvedTrack {
    pub stream_url: String,
    pub quality_hint: Option<String>,
    pub duration_ms: Option<u64>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderModule {
    pub id: String,
    pub name: String,
    pub layout: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleData {
    pub items: Vec<ModuleItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModuleItem {
    Track(TrackResult),
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
            if let Some(sts) = extract_signature_timestamp(&resp.body) {
                return Some((sts, resp.body));
            }
        }
    }
    None
}

fn get_or_refresh_sts() -> i64 {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

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
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
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
    
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
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
    os_name: Option<&'static str>,
    os_version: Option<&'static str>,
    device_make: Option<&'static str>,
    device_model: Option<&'static str>,
    android_sdk_version: Option<&'static str>,
}

fn fallback_clients() -> Vec<YtClient> {
    vec![
        YtClient {
            name: "VISIONOS", version: "0.1", client_id: "101",
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
            needs_potoken: false, needs_sts: false, include_ua_in_context: false,
            os_name: Some("visionOS"), os_version: Some("1.3.21O771"),
            device_make: Some("Apple"), device_model: Some("RealityDevice14,1"),
            android_sdk_version: None,
        },
        YtClient {
            name: "ANDROID_VR", version: "1.65.10", client_id: "28",
            user_agent: "com.google.android.apps.youtube.vr.oculus/1.65.10 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
            needs_potoken: false, needs_sts: false, include_ua_in_context: true,
            os_name: Some("Android"), os_version: Some("12L"),
            device_make: Some("Oculus"), device_model: Some("Quest 3"),
            android_sdk_version: Some("32"),
        },
        YtClient {
            name: "ANDROID_VR", version: "1.43.32", client_id: "28",
            user_agent: "com.google.android.apps.youtube.vr.oculus/1.43.32 (Linux; U; Android 12; en_US; Quest 3; Build/SQ3A.220605.009.A1; Cronet/107.0.5284.2)",
            needs_potoken: false, needs_sts: false, include_ua_in_context: true,
            os_name: Some("Android"), os_version: Some("12"),
            device_make: Some("Oculus"), device_model: Some("Quest 3"),
            android_sdk_version: Some("32"),
        },
        YtClient {
            name: "WEB_REMIX", version: "1.20260114.03.00", client_id: "67",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
            needs_potoken: true, needs_sts: true, include_ua_in_context: false,
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

    let mut body = serde_json::json!({
        "context": {
            "client": client_obj,
            "request": { "internalExperimentFlags": [], "useSsl": true },
            "user": { "lockedSafetyMode": false }
        },
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

fn do_search(input: String) -> FnResult<String> {
    let query: String = serde_json::from_str(&input)?;
    unsafe { host_log(format!("WASM searching for: {}", query))? };
    
    let vd = get_visitor_data()?;
    let body = serde_json::json!({
        "context": {
            "client": {
                "clientName": "WEB",
                "clientVersion": "2.20240101.01.00",
                "visitorData": vd
            }
        },
        "query": query
    });
    
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    
    let res = do_http("POST", "https://www.youtube.com/youtubei/v1/search", Some(headers), Some(body.to_string()))?;
    let json: serde_json::Value = serde_json::from_str(&res.body).unwrap_or_default();
    
    let mut tracks = Vec::new();
    // Simplified parsing of InnerTube search response
    if let Some(contents) = json["contents"]["twoColumnSearchResultsRenderer"]["primaryContents"]["sectionListRenderer"]["contents"].as_array() {
        for section in contents {
            if let Some(items) = section["itemSectionRenderer"]["contents"].as_array() {
                for item in items {
                    if let Some(video) = item.get("videoRenderer") {
                        let id = video["videoId"].as_str().unwrap_or("").to_string();
                        let title = video["title"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                        let artist = video["ownerText"]["runs"][0]["text"].as_str().unwrap_or("").to_string();
                        let duration = video["lengthText"]["simpleText"].as_str().unwrap_or("0:00").to_string();
                        
                        let parts: Vec<&str> = duration.split(':').collect();
                        let mut ms = 0;
                        if parts.len() == 2 {
                            ms = (parts[0].parse::<u64>().unwrap_or(0) * 60 + parts[1].parse::<u64>().unwrap_or(0)) * 1000;
                        }
                        
                        let cover = video["thumbnail"]["thumbnails"][0]["url"].as_str().map(|s| s.to_string());
                        
                        tracks.push(TrackResult {
                            id, title, artist, album: None, cover_art_url: cover, stream_url: None, quality_hint: None, duration_ms: Some(ms)
                        });
                    }
                }
            }
        }
    }
    
    Ok(serde_json::to_string(&tracks)?)
}

#[plugin_fn]
pub fn search(input: String) -> FnResult<String> {
    do_search(input)
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
        let mut best: Option<(&serde_json::Value, u8, i64)> = None;
        for format in formats {
            let mime = format["mimeType"].as_str().unwrap_or("");
            if !mime.starts_with("audio/") || format["url"].as_str().is_none() {
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
                Some((_, best_score, best_bitrate)) => (format_score, bitrate) > (best_score, best_bitrate),
            };
            if should_replace {
                best = Some((format, format_score, bitrate));
            }
        }
        let best = best.map(|(f, _, b)| (f, b));

        if let Some((format, _)) = best {
            stream_url = format["url"].as_str().unwrap_or("").to_string();
            quality_hint = format["audioQuality"].as_str().map(|s| s.to_string());
            duration_sec = json["videoDetails"]["lengthSeconds"].as_str().unwrap_or("0").parse().unwrap_or(0);
            unsafe { host_log(format!("[RESOLVE] Selected {} — audio format with direct URL", client.name))? };
            selected_client = Some(client);
            break;
        } else {
            unsafe { host_log(format!("[RESOLVE] {} OK but no directly-usable audio format (cipher required, unsupported)", client.name))? };
        }
    }

    let mut out_headers = HashMap::new();
    if let Some(ref client) = selected_client {
        out_headers.insert("User-Agent".to_string(), client.user_agent.to_string());
        if client.needs_potoken {
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
            id: "wasm_trending".to_string(),
            name: "YouTube WASM Trending".to_string(),
            layout: "Carousel".to_string(),
        }
    ];
    Ok(serde_json::to_string(&mods)?)
}

#[plugin_fn]
pub fn fetch_module(input: String) -> FnResult<String> {
    // Just wrap search for simplicity
    let res = do_search("\"top hits music\"".to_string())?;
    let tracks: Vec<TrackResult> = serde_json::from_str(&res).unwrap_or_default();
    let items = tracks.into_iter().map(ModuleItem::Track).collect();
    
    let data = ModuleData { items };
    Ok(serde_json::to_string(&data)?)
}
