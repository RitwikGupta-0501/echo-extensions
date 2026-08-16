# Echo Player: YouTube Production Checklist

To achieve the same level of daily production reliability as Metrolist, we need to implement several dynamic scraping and caching mechanisms. Hardcoding values (like we did for testing) will eventually break as YouTube updates its backend. 

Here is the checklist of systems Metrolist uses to ensure robust playback:

### 1. Dynamic `signatureTimestamp` Extraction
YouTube updates its `base.js` web player frequently (often every few days). The `signatureTimestamp` is essentially a version number for the obfuscation logic. 
- **Requirement:** We must implement a scraper that fetches the HTML of a YouTube watch page or `iframe`, locates the `base.js` player script URL, downloads it, and extracts the `signatureTimestamp` variable via Regex.
- **Why:** If the timestamp sent in the `PlayerBody` does not match the active version on YouTube's servers, BotGuard tokens will be rejected.

### 2. Cipher Deobfuscation (`signatureCipher`)
While some streams provide a direct `url` string, many high-quality streams return a `signatureCipher` string. This string contains a scrambled URL, an `s` (signature) parameter, and an `n` (throttle) parameter.
- **Requirement:** We need to port Metrolist's `CipherDeobfuscator`. This involves dynamically extracting Javascript functions from `base.js` that descramble the `s` and `n` parameters, and executing those functions in our JS VM to reconstruct the final stream URL.
- **Why:** Without this, many music videos and high-quality audio streams will be unplayable or aggressively speed-throttled.

### 3. Asynchronous VM Warmup
Initializing the BotGuard VM (fetching the snapshot, executing `GenerateIT`, and setting up the minter) takes anywhere from 2 to 5 seconds.
- **Requirement:** We should move BotGuard initialization to the application startup phase or run it asynchronously in the background *before* the user clicks play.
- **Why:** If we initialize BotGuard on-demand, every playback will have a noticeable 5-second delay.

### 4. Session & Token Caching
BotGuard initialization (`GenerateIT`) returns an `expirationTimeInSeconds` (usually a few hours). 
- **Requirement:** We must cache the initialized JS environment, the `visitorData` string, and the `poTokenMinter` Javascript callback. We should only re-run the heavy BotGuard initialization once the token approaches its expiration limit.
- **Why:** Re-running the entire BotGuard snapshot per track is extremely computationally expensive and can trigger YouTube's rate limits.

### 5. Client Fallback Matrix
Sometimes YouTube heavily restricts the `WEB_REMIX` client or flags certain IP addresses.
- **Requirement:** Metrolist implements a fallback mechanism. If `WEB_REMIX` returns `UNPLAYABLE`, they retry the Player API request using the `TVHTML5` or `ANDROID_VR` client definitions, which often have more lenient BotGuard enforcement.
- **Why:** Maximizes track availability.

### 6. Age-Restriction Bypasses
If a track is flagged as age-restricted, the `WEB_REMIX` client will refuse to return streaming data without user login cookies.
- **Requirement:** Metrolist detects age-restriction errors and immediately switches to a TV client (like `TVHTML5_SIMPLY`), which historically bypasses age restrictions without requiring a Google login. 
