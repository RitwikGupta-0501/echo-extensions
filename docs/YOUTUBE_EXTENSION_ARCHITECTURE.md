# Echo YouTube Extension: Architecture & Reverse-Engineering Guide

## Executive Summary: The 60-Second Mental Model

Streaming audio from YouTube Music without official API keys or user logins is notoriously difficult. Google employs an array of proprietary defenses—browser fingerprinting, encrypted signature ciphers, dynamic timestamps, and artificial bandwidth throttling—to prevent unauthorized scraping.

To solve this cleanly, reliably, and legally, Echo separates concerns across two clear subsystems:

```
┌────────────────────────────────────────────────────────┐
│                   Core App (Echo Core)                 │
│  - Generic local audio player & Rodio playback engine  │
│  - Zero scraper code bundled in core release           │
└──────────────────────────┬─────────────────────────────┘
                           │ Extism WASM Plugin Boundary
                           │ (resolve, search, warmup)
┌──────────────────────────▼─────────────────────────────┐
│              YouTube WASM Extension                    │
│  - Sandboxed Rust module (817 KB WebAssembly)          │
│  - Negotiates YouTube's 5 anti-scraping defenses       │
│  - Deciphers signatures & transforms bandwidth throttles│
└──────────────────────────┬─────────────────────────────┘
                           │ Headless JS Sandbox Bridge
┌──────────────────────────▼─────────────────────────────┐
│                 WebView Sandbox VM                     │
│  - Isolated JS runtime with strict CSP (no net access) │
│  - Runs Google's BotGuard VM & YouTube player closures │
└────────────────────────────────────────────────────────┘
```

1. **The Core App (`Echo Core`)**: Operates strictly as an audio player. It knows nothing about YouTube scraping algorithms, cipher math, or InnerTube payloads. It simply tells the extension: *"Give me an audio stream for track `dQw4w9WgXcQ`"*, and feeds the resulting stream into its audio pipeline.
2. **The YouTube WASM Extension (`youtube-wasm`)**: A self-contained, sandboxed WebAssembly module written in Rust. It encapsulates all YouTube-specific logic, manages cryptographic tokens, deciphers audio signatures, and transforms throttle parameters.
3. **The WebView Sandbox VM**: A headless, isolated JavaScript execution environment with a strict Content Security Policy (`connect-src 'none'`). It serves solely as an evaluation sandbox where Google's BotGuard VM and YouTube's closure-scoped player scripts can execute safely.

---

## 1. The "Why": YouTube's 5 Anti-Scraping Defenses & How Echo Solves Each

YouTube does not use a simple URL or a public API for media playback. Instead, it enforces a 5-layer gauntlet of checks. Understanding why each check exists is key to understanding the extension's design.

```
YouTube Defense Mechanism                      Echo WASM Solution
──────────────────────────────────────────────────────────────────────────────────────────────
1. Browser Fingerprinting (BotGuard VM)   -->  Dual-Token PoToken (Session vs. Video Scope)
2. Client Identity & Integrity            -->  Multi-Tier Client Emulation (Web Remix, VisionOS, TV)
3. Build-Time Timestamp Mismatch (STS)    -->  4-Tier Precedence Dynamic STS Extraction
4. Encrypted Signature Cipher             -->  Closure-Scoped Injection before '})(_yt_player);'
5. Bandwidth Throttling ('n' parameter)   -->  Closure-Scoped 'n' Class Transform & Query Rewrite
```

---

### Defense 1: Browser Fingerprinting & The BotGuard VM (PoToken)

#### The Problem
YouTube requires automated clients to prove they are running inside an authentic, human-operated web browser. It does this via **BotGuard (JNN)**—a proprietary JavaScript virtual machine (`bgProgram` / `bgVm`) delivered by Google. BotGuard inspects the environment (DOM properties, canvas rendering, timing anomalies) and computes a cryptographic Proof-of-Origin Token (**PoToken**).

Without a valid PoToken:
- Requests to `/youtubei/v1/player` return `LOGIN_REQUIRED` or `SIGN_IN_TO_CONFIRM_YOU_ARE_NOT_A_BOT`.
- Media URLs are rejected by YouTube CDN edge nodes with HTTP 403 Forbidden.

#### The Echo Solution: The Dual-Token Architecture
Echo's extension implements a two-tiered token lifecycle:
1. **Session-Bound PoToken (Request Body Token)**:
   - Minted against the client's `visitorData` string (`Cgt...`).
   - Passed inside the InnerTube `/player` JSON payload under `serviceIntegrityDimensions.poToken`.
   - **Critical Rule**: This token is minted **once** at initialization and cached in storage. Re-minting it on every song triggers rate-limiting from Google's attestation servers.
2. **Video-Bound PoToken (CDN Stream Token)**:
   - Minted against the specific YouTube `videoId`.
   - Appended directly to the final media playback URL as `&pot=<token>`.
   - Validated by Google's `googlevideo.com` CDN edge servers when streaming audio chunks.

---

### Defense 2: Client Identity & InnerTube Matrix

#### The Problem
YouTube's internal API (`InnerTube`) serves different formats and enforces different security policies depending on the client identity declared in the request header (`x-youtube-client-name` and `context.client`).
- `WEB_REMIX` (YouTube Music Web): Delivers pristine Opus audio (itag 249/250/251, ~160kbps), but strictly enforces BotGuard, STS, and cipher transforms.
- `VISIONOS` & `ANDROID_VR`: Deliver direct, unencrypted stream URLs without cipher obfuscation, but cannot play age-restricted, explicit, or region-restricted songs.
- `TVHTML5`: Can stream age-restricted content without an account, but uses different codec allocations.

#### The Echo Solution: Priority Fallback Pipeline
The extension iterates through an ordered hierarchy of clients:
```
WEB_REMIX (Primary high-bitrate Opus)
   ↓ (if restricted or unavailable)
VISIONOS (Direct URL fallback)
   ↓ (if unplayable)
ANDROID_VR (Direct URL fallback)
   ↓ (if age-restricted)
TVHTML5_SIMPLY_EMBEDDED_PLAYER
```
If a track requires modern cipher deobfuscation (`WEB_REMIX`), the extension seamlessly handles it. If that fails, it cascades down to direct-stream clients, ensuring near-100% playback reliability.

---

### Defense 3: Dynamic Signature Timestamp (STS)

#### The Problem
Inside YouTube's `/youtubei/v1/player` request payload, Google expects a numeric field called `signatureTimestamp` (STS):
```json
{
  "playbackContext": {
    "contentPlaybackContext": {
      "signatureTimestamp": 20697
    }
  }
}
```
The STS represents the build day/epoch of the active `player_ias.vflset/base.js` compiler. If the STS sent in the request does not match the player generation currently deciphering the stream, the CDN instantly returns HTTP 403 Forbidden. Hardcoding this number guarantees playback will break within a few days when YouTube rotates players.

#### The Echo Solution: 4-Tier STS Resolution
The extension determines the current STS using a strict 4-tier precedence chain (derived from Metrolist):
1. **Anchored Literal from `base.js`**: Parses the live `signatureTimestamp:(\d+)` embedded in the active player JavaScript.
2. **Validated Remote Config (`player_configs.json`)**: Queries the Zemer/Metrolist player configuration table for the active player hash.
3. **Loose Regex Extraction**: Looks for loose `sts:(\d+)` patterns in the player script.
4. **Stale Cache / Default Constant**: Uses the last verified working timestamp as a fallback.

---

### Defense 4: Encrypted Signatures & The "Closure Scope" Revelation

#### The Problem
High-bitrate audio formats on YouTube Music do not provide a direct `url`. Instead, they return a `signatureCipher` query string:
```
s=m3u8...&sp=sig&url=https://rr1---sn-4g5ednle.googlevideo.com/videoplayback?...
```
To stream the audio, the obfuscated signature `s` must be transformed into a valid `sig` parameter.

In previous years, scrapers solved this by extracting small helper functions (like array reverse or splice) with regular expressions. However, in modern YouTube players (2025/2026+), YouTube introduced **VM-dispatch state machines** and **Q-array obfuscation**:
```javascript
var Q = "a}b}c}...".split("}");
function rY(a, b, c) { /* 500 lines of state machine code */ }
```
Crucially, YouTube wraps the entire `base.js` file inside an anonymous Immediately Invoked Function Expression (IIFE):
```javascript
var _yt_player = {};
(function(g) {
    var window = this;
    // rY and thousands of functions live ONLY in this private local closure!
    function rY(a, b, c) { ... }
})(_yt_player);
```
**Why naive `eval()` fails**: If you load `base.js` and then try to evaluate `rY(18, 3016, sig)` from outside, the JavaScript engine throws:
`ReferenceError: rY is not defined`.
The deciphering function is physically invisible outside the private closure.

#### The Echo Solution: The Closure Injection Technique
Rather than attempting to parse 3 megabytes of obfuscated JavaScript with brittle regex engines, Echo injects wrapper functions **inside the closure before it closes**:

```
Original YouTube base.js:
┌────────────────────────────────────────────────────────┐
│  var _yt_player = {};                                  │
│  (function(g) {                                        │
│      var window = this;                                │
│      function rY(a, b, c) { ... }                      │
│                                                        │
│      // <--- ECHO INJECTS EXPORTS RIGHT HERE!          │
│  })(_yt_player);                                       │
└────────────────────────────────────────────────────────┘
```

The extension replaces the closing string `})(_yt_player);` with:
```javascript
; window.__yt_sig_decipher = window._cipherSigFunc = function(sig) {
    try {
        return rY(18, 3016, sig);
    } catch(e) {
        return null;
    }
};
window.__yt_n_transform = window._nTransformFunc = function(n) {
    try {
        var u = new g.um('https://x.googlevideo.com/videoplayback?n=' + n, true);
        var t = u.get('n');
        return (t && t !== n) ? t : n;
    } catch(e) {
        return n;
    }
};
window.__yt_loaded_player_hash = 'f572e43c';
})(_yt_player);
```
Because the export functions are defined *inside* the closure, they have native access to `rY`, `g`, and all local variables. Once evaluated in the WebView Sandbox VM, `window.__yt_sig_decipher(s)` can be called directly from WebAssembly.

---

### Defense 5: The 'n' Parameter Bandwidth Throttle

#### The Problem
Even if a client successfully deciphers the signature and constructs a valid streaming URL, YouTube applies a subtle, aggressive bandwidth throttle. Every stream URL contains an `n` query parameter:
```
https://...googlevideo.com/videoplayback?...&n=RFPww1wkPDesRj7vlj&...
```
If the client plays the URL with the raw `n` parameter, Google's CDN throttles download speeds to **40–60 kbps**. Songs start playing, but within 3–5 seconds the audio buffer starves, causing continuous stuttering or complete stalls.

#### The Echo Solution: Closure-Scoped Class Transformation
In modern players, the `n` parameter is calculated by instantiating a specific closure class (`g.um`, `g.np`, `g.ty`, etc.) and retrieving its transformed property:
```javascript
var u = new g.um('https://x.googlevideo.com/videoplayback?n=' + n, true);
var transformedN = u.get('n');
```
Our closure injection wrapper automatically runs this transformation:
```
Input n:        RFPww1wkPDesRj7vlj
Transformed n:  sIKXvenvrP1LBA
```
The extension replaces `n=<old>` with `n=<new>` in the streaming URL. As a result, the CDN releases the throttle, allowing full-speed streaming (HTTP 206 Partial Content) with zero buffering delays.

---

## 2. Remote Self-Healing & Player Rotation Resilience

YouTube deploys new player versions every 48 to 72 hours, changing function names (`rY` $\rightarrow$ `WQ`, `um` $\rightarrow$ `ty`). 

To prevent player rotations from breaking playback, Echo employs a **3-tier self-healing system**:

1. **Remote Configuration Cache**: The extension synchronizes with the upstream Zemer/Metrolist cipher repository (`player_configs.json`). It caches the table in host storage with a 6-hour TTL (`ZEMER_CONFIG_TTL_SECS = 21600`).
2. **Embedded Offline Table**: The WASM binary embeds verified configurations for recent player generations (`f572e43c`, `e2a2364f`, `ab18ea88`, etc.). If the device is offline or GitHub raw is unreachable, it resolves immediately without network lookups.
3. **Mid-Session Force Refresh**: If YouTube rotates to a new player hash not present in cache, the extension detects the unknown hash, bypasses the TTL, forces a fresh fetch from the remote store, and continues playback without requiring the user to restart the app or install an update.

---

## 3. Step-by-Step Request Lifecycle & Sequence Diagram

The interaction across subsystems follows a clean, decoupled boundary:

```mermaid
sequenceDiagram
    autonumber
    participant Core as Core App (Echo Core)
    participant WASM as YouTube WASM Extension
    participant Sandbox as WebView Sandbox VM
    participant YT as YouTube (InnerTube & CDN)

    Core->>WASM: resolve(track_id)
    activate WASM
    
    WASM->>Sandbox: Init BotGuard & mint session PoToken
    Sandbox-->>WASM: session_potoken
    
    WASM->>Sandbox: Mint video-bound PoToken(track_id)
    Sandbox-->>WASM: video_potoken
    
    WASM->>YT: GET iframe_api (discover active player hash)
    YT-->>WASM: Player hash (e.g. f572e43c)
    
    WASM->>WASM: Resolve STS & lookup cipher config
    
    WASM->>YT: POST /youtubei/v1/player (client: WEB_REMIX, STS, session_potoken)
    YT-->>WASM: PlayerResponse (formats, signatureCipher)
    
    WASM->>YT: GET player_ias.vflset/base.js
    YT-->>WASM: player.js (raw script)
    
    WASM->>WASM: Inject exports before '})(_yt_player);'
    WASM->>Sandbox: Execute modified player.js
    Sandbox-->>WASM: Evaluation ready (window.__yt_sig_decipher verified)
    
    WASM->>Sandbox: Execute window.__yt_sig_decipher(obfuscated_s)
    Sandbox-->>WASM: Deciphered signature (len: 104)
    
    WASM->>Sandbox: Execute window.__yt_n_transform(raw_n)
    Sandbox-->>WASM: Transformed n parameter
    
    WASM->>WASM: Assemble stream URL + &pot=video_potoken
    WASM->>WASM: Attach headers (Origin: music.youtube.com)
    
    WASM-->>Core: Return ResolvedTrack (stream_url, headers, duration)
    deactivate WASM
    
    Core->>YT: HTTP GET / Range Request (206 Partial Content)
    YT-->>Core: Audio stream chunks (audio/webm; codecs="opus")
    Core->>Core: Decode & stream via Rodio audio engine
```

---

## 4. Code Anatomy & Host Interface Contract

### The Extension Source: `extensions/youtube-wasm/src/lib.rs`
The extension compiles to a single, lightweight WebAssembly binary (`target/wasm32-wasip1/release/youtube_wasm.wasm`, **817 KB**). It uses Extism PDK without bloated regex state machines.

Key entry points:
- `pub fn resolve(input: String) -> FnResult<String>`: The main playback resolver.
- `pub fn search(input: String) -> FnResult<String>`: InnerTube search query parser.
- `pub fn warmup(_input: String) -> FnResult<String>`: Pre-warms BotGuard and player caches ahead of first playback.
- `pub fn get_modules() -> FnResult<String>`: Returns category shelves and discover modules for the Explore page.

### The Host Interface Contract
The WASM extension communicates with the Core App strictly through five safe host functions:

| Host Function | Signature | Purpose |
| :--- | :--- | :--- |
| `host_log` | `(String) -> ()` | Forwards structured log messages to the Core App's tracing system. |
| `host_storage_get` | `(String) -> Option<String>` | Reads cached tokens, player hashes, and STS from host key-value storage. |
| `host_storage_set` | `(String, String) -> ()` | Persists session tokens and configs across app restarts. |
| `host_http_request` | `(HttpRequest) -> HttpResponse` | Executes outbound network requests via the Core App's Tokio `reqwest` client. |
| `host_execute_webview_js` | `(String) -> String` | Sends JavaScript to the headless WebView Sandbox VM and returns the evaluated string. |

---

## 5. Verification & Testing Playbook

### Verifying Against Real Tracks
To verify that the extension is working without running the entire desktop UI, execute the standalone test harness:

```bash
node scratch/test_yt_resolver.js
```

The harness runs live requests against YouTube CDN endpoints for 3 diverse tracks:
1. `dQw4w9WgXcQ` (*Never Gonna Give You Up*): Tests signature cipher deobfuscation and Opus audio stream validation.
2. `kJQP7kiw5Fk` (*Despacito*): Tests signature deciphering combined with N-parameter transform verification.
3. `2Vv-BfVoq4g` (*Perfect*): Tests standard cipher resolution and stream stability.

**Successful Verification Criteria:**
- Output reports `Deciphered signature: len 104` (from raw length 114).
- Output reports `Transformed 'n' parameter: <old> -> <new>`.
- The CDN response returns **`HTTP 206 | Length: 1025 bytes | Type: audio/webm`**.

---

## 6. Summary Checklist for Explaining to Others

If you need to explain how this works in a discussion or interview, remember these three core points:

1. **Why standard scrapers fail**: YouTube blocks direct scraping using BotGuard environment fingerprinting, scrambles signatures inside anonymous JavaScript closures, and artificially throttles download bandwidth if the 'n' parameter isn't transformed.
2. **How Echo solves it**: The extension injects export wrappers directly inside YouTube's player closure (`})(_yt_player);`) before evaluation, enabling native execution of decipher and un-throttle routines, while using BotGuard PoTokens for request attestation.
3. **Why it is structured this way**: Isolating the scraper in a WebAssembly module keeps the Core App clean, generic, and legally compliant, while dynamic remote config caching ensures the extension never breaks when YouTube rotates its player code.
