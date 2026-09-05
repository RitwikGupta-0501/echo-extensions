use extism_pdk::*;
use std::collections::HashMap;
use base64::{Engine as _, engine::general_purpose};
use crate::models::HttpResponse;
use crate::host::{
    do_http, host_log, host_execute_webview_js,
    get_storage, set_storage, get_unix_timestamp,
};

pub const REQUEST_KEY: &str = "O43z0dpjhgX20SCx4KAo";
pub const GOOGLE_API_KEY: &str = "AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw";
pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.3";

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


pub fn init_botguard() -> FnResult<bool> {
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


pub fn generate_po_token(identifier: &str) -> FnResult<String> {
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


pub fn find_visitor_data_in_json(val: &serde_json::Value) -> Option<String> {
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


pub fn get_visitor_data() -> FnResult<String> {
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


