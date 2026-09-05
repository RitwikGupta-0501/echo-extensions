use extism_pdk::*;
use std::collections::HashMap;
use crate::models::{HttpRequest, HttpResponse, StorageRequest};

#[host_fn]
extern "ExtismHost" {
    pub fn host_execute_webview_js(script: String) -> String;
    pub fn host_log(msg: String);
    pub fn host_http_request(req: String) -> String;
    pub fn host_telemetry_request(req: String) -> String;
    pub fn host_storage_get(req: String) -> String;
    pub fn host_storage_set(req: String);
}

pub fn do_http(method: &str, url: &str, headers: Option<HashMap<String, String>>, body: Option<String>) -> FnResult<HttpResponse> {
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


pub fn get_storage(key: &str) -> FnResult<String> {
    let req = StorageRequest {
        provider_id: "youtube-wasm".to_string(),
        key: key.to_string(),
        value: None,
    };
    let json = serde_json::to_string(&req)?;
    let res = unsafe { host_storage_get(json)? };
    Ok(res)
}


pub fn set_storage(key: &str, value: &str) -> FnResult<()> {
    let req = StorageRequest {
        provider_id: "youtube-wasm".to_string(),
        key: key.to_string(),
        value: Some(value.to_string()),
    };
    let json = serde_json::to_string(&req)?;
    unsafe { host_storage_set(json)? };
    Ok(())
}


pub fn get_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1700000000)
}


pub fn url_decode_component(s: &str) -> String {
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


pub fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}



