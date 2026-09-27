use super::{Geolocation, HttpCredentials, StorageState};
use crate::api::launch_options::IgnoreDefaultArgs;
use crate::protocol::ProxySettings;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Viewport dimensions for browser context.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    /// Page width in pixels
    pub width: u32,
    /// Page height in pixels
    pub height: u32,
}

/// Options for recording HAR.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-record-har>
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RecordHar {
    /// Path on the filesystem to write the HAR file to.
    pub path: String,
    /// Optional setting to control whether to omit request content from the HAR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omit_content: Option<bool>,
    /// Optional setting to control resource content management.
    /// "omit" | "embed" | "attach"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// "full" | "minimal"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// A glob or regex pattern to filter requests that are stored in the HAR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_filter: Option<String>,
}

impl RecordHar {
    /// Record a HAR to the given path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            omit_content: None,
            content: None,
            mode: None,
            url_filter: None,
        }
    }
    /// Omit response bodies from the HAR.
    pub fn omit_content(mut self, omit_content: bool) -> Self {
        self.omit_content = Some(omit_content);
        self
    }
    /// Content mode ("embed", "attach", or "omit").
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }
    /// Recording mode ("full" or "minimal").
    pub fn mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = Some(mode.into());
        self
    }
    /// Only record requests matching this URL glob.
    pub fn url_filter(mut self, url_filter: impl Into<String>) -> Self {
        self.url_filter = Some(url_filter.into());
        self
    }
}

/// Options for recording video.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-record-video>
#[derive(Debug, Clone, Serialize, Default)]
#[non_exhaustive]
pub struct RecordVideo {
    /// Path to the directory to put videos into.
    pub dir: String,
    /// Optional dimensions of the recorded videos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Viewport>,
}

impl RecordVideo {
    /// Record videos into the given directory.
    pub fn new(dir: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            size: None,
        }
    }
    /// Recorded video size.
    pub fn size(mut self, size: Viewport) -> Self {
        self.size = Some(size);
        self
    }
}

/// Controls how downloads are handled in a [`BrowserContext`](super::BrowserContext).
///
/// See the `accept_downloads` field of [`BrowserContextOptions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum AcceptDownloads {
    /// Allow and capture downloads via the `download` event.
    #[serde(rename = "accept")]
    Accept,
    /// Block downloads.
    #[serde(rename = "deny")]
    Deny,
    /// Let the browser handle downloads natively without routing through Playwright.
    #[serde(rename = "internal")]
    Internal,
}

impl From<bool> for AcceptDownloads {
    fn from(value: bool) -> Self {
        if value { Self::Accept } else { Self::Deny }
    }
}

/// Options for creating a new browser context.
///
/// Allows customizing viewport, user agent, locale, timezone, geolocation,
/// permissions, and other browser context settings.
///
/// A proxy and a saved session, built inline; `storage_state_path` loads the
/// same state from a file written by `BrowserContext::storage_state`:
///
/// ```rust
/// use playwright_rs::protocol::{
///     BrowserContextOptions, Cookie, LocalStorageItem, Origin, ProxySettings, StorageState,
/// };
///
/// let session = StorageState::default()
///     .cookies(vec![
///         Cookie::new("session_id", "abc123")
///             .domain(".example.com")
///             .http_only(true)
///             .secure(true)
///             .same_site("Lax"),
///     ])
///     .origins(vec![Origin::new(
///         "https://example.com",
///         vec![LocalStorageItem::new("user_prefs", "{\"theme\":\"dark\"}")],
///     )]);
///
/// let options = BrowserContextOptions::builder()
///     .proxy(
///         ProxySettings::new("http://proxy.example.com:8080")
///             .bypass(".example.com")
///             .username("user")
///             .password("pass"),
///     )
///     .storage_state(session)
///     .build();
/// ```
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BrowserContextOptions {
    /// Sets consistent viewport for all pages in the context.
    /// Set to null via `no_viewport(true)` to disable viewport emulation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,

    /// Disables viewport emulation when set to true.
    /// Note: Playwright's public API calls this `noViewport`, but the protocol
    /// expects `noDefaultViewport`. playwright-python applies this transformation
    /// in `_prepare_browser_context_params`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "noDefaultViewport")]
    pub no_viewport: Option<bool>,

    /// Custom user agent string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// Locale for the context (e.g., "en-GB", "de-DE", "fr-FR")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    /// Timezone identifier (e.g., "America/New_York", "Europe/Berlin")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone_id: Option<String>,

    /// Geolocation coordinates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<Geolocation>,

    /// Credentials for HTTP authentication, matched per request origin.
    #[serde(rename = "httpCredentials", skip_serializing_if = "Option::is_none")]
    pub http_credentials: Option<Vec<HttpCredentials>>,

    /// List of permissions to grant (e.g., "geolocation", "notifications")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,

    /// Network proxy settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy: Option<ProxySettings>,

    /// Emulates 'prefers-colors-scheme' media feature ("light", "dark", "no-preference")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,

    /// Whether the viewport supports touch events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_touch: Option<bool>,

    /// Whether the meta viewport tag is respected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_mobile: Option<bool>,

    /// Whether JavaScript is enabled in the context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub javascript_enabled: Option<bool>,

    /// Emulates network being offline
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offline: Option<bool>,

    /// How to handle downloads. See [`AcceptDownloads`] for options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_downloads: Option<AcceptDownloads>,

    /// Whether to bypass Content-Security-Policy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bypass_csp: Option<bool>,

    /// Whether to ignore HTTPS errors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_https_errors: Option<bool>,

    /// Device scale factor (default: 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_scale_factor: Option<f64>,

    /// Extra HTTP headers to send with every request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_http_headers: Option<HashMap<String, String>>,

    /// Base URL for relative navigation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Storage state to populate the context (cookies, localStorage, sessionStorage).
    /// Can be an inline StorageState object or a file path string.
    /// Use builder methods `storage_state()` for inline or `storage_state_path()` for file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_state: Option<StorageState>,

    /// Storage state file path (alternative to inline storage_state).
    /// This is handled by the builder and converted to storage_state during serialization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_state_path: Option<String>,

    // Launch options (for launch_persistent_context)
    /// Additional arguments to pass to browser instance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,

    /// Browser distribution channel (e.g., "chrome", "msedge")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    /// Enable Chromium sandboxing (default: false on Linux)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromium_sandbox: Option<bool>,

    /// Auto-open DevTools (deprecated, default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devtools: Option<bool>,

    /// Directory to save downloads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub downloads_path: Option<String>,

    /// Path to custom browser executable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,

    /// Firefox user preferences (Firefox only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firefox_user_prefs: Option<HashMap<String, serde_json::Value>>,

    /// Run in headless mode (default: true unless devtools=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headless: Option<bool>,

    /// Filter or disable default browser arguments.
    /// When `true`, Playwright does not pass its own default args.
    /// When an array, filters out the given default arguments.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_default_args: Option<IgnoreDefaultArgs>,

    /// Slow down operations by N milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_mo: Option<f64>,

    /// Timeout for browser launch in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,

    /// Directory to save traces
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traces_dir: Option<String>,

    /// Check if strict selectors mode is enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict_selectors: Option<bool>,

    /// Emulates 'prefers-reduced-motion' media feature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduced_motion: Option<String>,

    /// Emulates 'forced-colors' media feature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forced_colors: Option<String>,

    /// Whether to allow sites to register Service workers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_workers: Option<String>,

    /// Options for recording HAR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_har: Option<RecordHar>,

    /// Options for recording video
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_video: Option<RecordVideo>,
}

impl BrowserContextOptions {
    /// Creates a new builder for BrowserContextOptions
    pub fn builder() -> BrowserContextOptionsBuilder {
        BrowserContextOptionsBuilder::default()
    }
}

/// Builder for BrowserContextOptions
#[derive(Debug, Clone, Default)]
pub struct BrowserContextOptionsBuilder {
    viewport: Option<Viewport>,
    no_viewport: Option<bool>,
    user_agent: Option<String>,
    locale: Option<String>,
    timezone_id: Option<String>,
    geolocation: Option<Geolocation>,
    http_credentials: Option<Vec<HttpCredentials>>,
    permissions: Option<Vec<String>>,
    proxy: Option<ProxySettings>,
    color_scheme: Option<String>,
    has_touch: Option<bool>,
    is_mobile: Option<bool>,
    javascript_enabled: Option<bool>,
    offline: Option<bool>,
    accept_downloads: Option<AcceptDownloads>,
    bypass_csp: Option<bool>,
    ignore_https_errors: Option<bool>,
    device_scale_factor: Option<f64>,
    extra_http_headers: Option<HashMap<String, String>>,
    base_url: Option<String>,
    storage_state: Option<StorageState>,
    storage_state_path: Option<String>,
    // Launch options
    args: Option<Vec<String>>,
    channel: Option<String>,
    chromium_sandbox: Option<bool>,
    devtools: Option<bool>,
    downloads_path: Option<String>,
    executable_path: Option<String>,
    firefox_user_prefs: Option<HashMap<String, serde_json::Value>>,
    headless: Option<bool>,
    ignore_default_args: Option<IgnoreDefaultArgs>,
    slow_mo: Option<f64>,
    timeout: Option<f64>,
    traces_dir: Option<String>,
    strict_selectors: Option<bool>,
    reduced_motion: Option<String>,
    forced_colors: Option<String>,
    service_workers: Option<String>,
    record_har: Option<RecordHar>,
    record_video: Option<RecordVideo>,
}

impl BrowserContextOptionsBuilder {
    /// Sets the viewport dimensions
    pub fn viewport(mut self, viewport: Viewport) -> Self {
        self.viewport = Some(viewport);
        self.no_viewport = None; // Clear no_viewport if setting viewport
        self
    }

    /// Disables viewport emulation
    pub fn no_viewport(mut self, no_viewport: bool) -> Self {
        self.no_viewport = Some(no_viewport);
        if no_viewport {
            self.viewport = None; // Clear viewport if setting no_viewport
        }
        self
    }

    /// Sets the user agent string
    pub fn user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Sets the locale
    pub fn locale(mut self, locale: String) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Sets the timezone identifier
    pub fn timezone_id(mut self, timezone_id: String) -> Self {
        self.timezone_id = Some(timezone_id);
        self
    }

    /// Sets the geolocation
    pub fn geolocation(mut self, geolocation: Geolocation) -> Self {
        self.geolocation = Some(geolocation);
        self
    }

    /// Sets credentials for HTTP authentication.
    ///
    /// Each request uses the first entry whose `origin` matches it; an entry
    /// without an origin matches anything.
    pub fn http_credentials(mut self, credentials: Vec<HttpCredentials>) -> Self {
        self.http_credentials = Some(credentials);
        self
    }

    /// Sets the permissions to grant
    pub fn permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Sets the network proxy settings for this context.
    ///
    /// This allows routing all network traffic through a proxy server,
    /// useful for rotating proxies without creating new browsers.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
    pub fn proxy(mut self, proxy: ProxySettings) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Sets the color scheme preference
    pub fn color_scheme(mut self, color_scheme: String) -> Self {
        self.color_scheme = Some(color_scheme);
        self
    }

    /// Sets whether the viewport supports touch events
    pub fn has_touch(mut self, has_touch: bool) -> Self {
        self.has_touch = Some(has_touch);
        self
    }

    /// Sets whether this is a mobile viewport
    pub fn is_mobile(mut self, is_mobile: bool) -> Self {
        self.is_mobile = Some(is_mobile);
        self
    }

    /// Sets whether JavaScript is enabled
    pub fn javascript_enabled(mut self, javascript_enabled: bool) -> Self {
        self.javascript_enabled = Some(javascript_enabled);
        self
    }

    /// Sets whether to emulate offline network
    pub fn offline(mut self, offline: bool) -> Self {
        self.offline = Some(offline);
        self
    }

    /// Sets how to handle downloads. Accepts `AcceptDownloads` or `bool`
    /// (`true` → `Accept`, `false` → `Deny`).
    pub fn accept_downloads(mut self, accept_downloads: impl Into<AcceptDownloads>) -> Self {
        self.accept_downloads = Some(accept_downloads.into());
        self
    }

    /// Sets whether to bypass Content-Security-Policy
    pub fn bypass_csp(mut self, bypass_csp: bool) -> Self {
        self.bypass_csp = Some(bypass_csp);
        self
    }

    /// Sets whether to ignore HTTPS errors
    pub fn ignore_https_errors(mut self, ignore_https_errors: bool) -> Self {
        self.ignore_https_errors = Some(ignore_https_errors);
        self
    }

    /// Sets the device scale factor
    pub fn device_scale_factor(mut self, device_scale_factor: f64) -> Self {
        self.device_scale_factor = Some(device_scale_factor);
        self
    }

    /// Sets extra HTTP headers
    pub fn extra_http_headers(mut self, extra_http_headers: HashMap<String, String>) -> Self {
        self.extra_http_headers = Some(extra_http_headers);
        self
    }

    /// Sets the base URL for relative navigation
    pub fn base_url(mut self, base_url: String) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Sets the storage state inline (cookies, localStorage).
    ///
    /// Populates the browser context with the provided storage state, including
    /// cookies and local storage. This is useful for initializing a context with
    /// a saved authentication state.
    ///
    /// Mutually exclusive with `storage_state_path()`.
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
    pub fn storage_state(mut self, storage_state: StorageState) -> Self {
        self.storage_state = Some(storage_state);
        self.storage_state_path = None; // Clear path if setting inline
        self
    }

    /// Sets the storage state from a file path.
    ///
    /// The file should contain a JSON representation of StorageState with cookies
    /// and origins. This is useful for loading authentication state saved from a
    /// previous session.
    ///
    /// Mutually exclusive with `storage_state()`.
    ///
    /// The file should have this format:
    /// ```json
    /// {
    ///   "cookies": [{
    ///     "name": "session_id",
    ///     "value": "abc123",
    ///     "domain": ".example.com",
    ///     "path": "/",
    ///     "expires": -1,
    ///     "httpOnly": true,
    ///     "secure": true,
    ///     "sameSite": "Lax"
    ///   }],
    ///   "origins": [{
    ///     "origin": "https://example.com",
    ///     "localStorage": [{
    ///       "name": "user_prefs",
    ///       "value": "{\"theme\":\"dark\"}"
    ///     }]
    ///   }]
    /// }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
    pub fn storage_state_path(mut self, path: String) -> Self {
        self.storage_state_path = Some(path);
        self.storage_state = None; // Clear inline if setting path
        self
    }

    /// Sets additional arguments to pass to browser instance (for launch_persistent_context)
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = Some(args);
        self
    }

    /// Sets browser distribution channel (for launch_persistent_context)
    pub fn channel(mut self, channel: String) -> Self {
        self.channel = Some(channel);
        self
    }

    /// Enables or disables Chromium sandboxing (for launch_persistent_context)
    pub fn chromium_sandbox(mut self, enabled: bool) -> Self {
        self.chromium_sandbox = Some(enabled);
        self
    }

    /// Auto-open DevTools (for launch_persistent_context)
    pub fn devtools(mut self, enabled: bool) -> Self {
        self.devtools = Some(enabled);
        self
    }

    /// Sets directory to save downloads (for launch_persistent_context)
    pub fn downloads_path(mut self, path: String) -> Self {
        self.downloads_path = Some(path);
        self
    }

    /// Sets path to custom browser executable (for launch_persistent_context)
    pub fn executable_path(mut self, path: String) -> Self {
        self.executable_path = Some(path);
        self
    }

    /// Sets Firefox user preferences (for launch_persistent_context, Firefox only)
    pub fn firefox_user_prefs(mut self, prefs: HashMap<String, serde_json::Value>) -> Self {
        self.firefox_user_prefs = Some(prefs);
        self
    }

    /// Run in headless mode (for launch_persistent_context)
    pub fn headless(mut self, enabled: bool) -> Self {
        self.headless = Some(enabled);
        self
    }

    /// Filter or disable default browser arguments (for launch_persistent_context).
    ///
    /// When `IgnoreDefaultArgs::Bool(true)`, Playwright does not pass its own
    /// default arguments and only uses the ones from `args`.
    /// When `IgnoreDefaultArgs::Array(vec)`, filters out the given default arguments.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsertype#browser-type-launch-persistent-context>
    pub fn ignore_default_args(mut self, args: IgnoreDefaultArgs) -> Self {
        self.ignore_default_args = Some(args);
        self
    }

    /// Slow down operations by N milliseconds (for launch_persistent_context)
    pub fn slow_mo(mut self, ms: f64) -> Self {
        self.slow_mo = Some(ms);
        self
    }

    /// Set timeout for browser launch in milliseconds (for launch_persistent_context)
    pub fn timeout(mut self, ms: f64) -> Self {
        self.timeout = Some(ms);
        self
    }

    /// Set directory to save traces (for launch_persistent_context)
    pub fn traces_dir(mut self, path: String) -> Self {
        self.traces_dir = Some(path);
        self
    }

    /// Check if strict selectors mode is enabled
    pub fn strict_selectors(mut self, enabled: bool) -> Self {
        self.strict_selectors = Some(enabled);
        self
    }

    /// Emulates 'prefers-reduced-motion' media feature
    pub fn reduced_motion(mut self, value: String) -> Self {
        self.reduced_motion = Some(value);
        self
    }

    /// Emulates 'forced-colors' media feature
    pub fn forced_colors(mut self, value: String) -> Self {
        self.forced_colors = Some(value);
        self
    }

    /// Whether to allow sites to register Service workers ("allow" | "block")
    pub fn service_workers(mut self, value: String) -> Self {
        self.service_workers = Some(value);
        self
    }

    /// Sets options for recording HAR
    pub fn record_har(mut self, record_har: RecordHar) -> Self {
        self.record_har = Some(record_har);
        self
    }

    /// Sets options for recording video
    pub fn record_video(mut self, record_video: RecordVideo) -> Self {
        self.record_video = Some(record_video);
        self
    }

    /// Builds the BrowserContextOptions
    pub fn build(self) -> BrowserContextOptions {
        BrowserContextOptions {
            viewport: self.viewport,
            no_viewport: self.no_viewport,
            user_agent: self.user_agent,
            locale: self.locale,
            timezone_id: self.timezone_id,
            geolocation: self.geolocation,
            http_credentials: self.http_credentials,
            permissions: self.permissions,
            proxy: self.proxy,
            color_scheme: self.color_scheme,
            has_touch: self.has_touch,
            is_mobile: self.is_mobile,
            javascript_enabled: self.javascript_enabled,
            offline: self.offline,
            accept_downloads: self.accept_downloads,
            bypass_csp: self.bypass_csp,
            ignore_https_errors: self.ignore_https_errors,
            device_scale_factor: self.device_scale_factor,
            extra_http_headers: self.extra_http_headers,
            base_url: self.base_url,
            storage_state: self.storage_state,
            storage_state_path: self.storage_state_path,
            // Launch options
            args: self.args,
            channel: self.channel,
            chromium_sandbox: self.chromium_sandbox,
            devtools: self.devtools,
            downloads_path: self.downloads_path,
            executable_path: self.executable_path,
            firefox_user_prefs: self.firefox_user_prefs,
            headless: self.headless,
            ignore_default_args: self.ignore_default_args,
            slow_mo: self.slow_mo,
            timeout: self.timeout,
            traces_dir: self.traces_dir,
            strict_selectors: self.strict_selectors,
            reduced_motion: self.reduced_motion,
            forced_colors: self.forced_colors,
            service_workers: self.service_workers,
            record_har: self.record_har,
            record_video: self.record_video,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_context_options_ignore_default_args_bool_serialization() {
        let options = BrowserContextOptions::builder()
            .ignore_default_args(IgnoreDefaultArgs::Bool(true))
            .build();

        let value = serde_json::to_value(&options).unwrap();
        assert_eq!(value["ignoreDefaultArgs"], serde_json::json!(true));
    }

    #[test]
    fn test_browser_context_options_ignore_default_args_array_serialization() {
        let options = BrowserContextOptions::builder()
            .ignore_default_args(IgnoreDefaultArgs::Array(vec!["--foo".to_string()]))
            .build();

        let value = serde_json::to_value(&options).unwrap();
        assert_eq!(value["ignoreDefaultArgs"], serde_json::json!(["--foo"]));
    }

    #[test]
    fn test_browser_context_options_ignore_default_args_absent() {
        let options = BrowserContextOptions::builder().build();

        let value = serde_json::to_value(&options).unwrap();
        assert!(value.get("ignoreDefaultArgs").is_none());
    }

    #[test]
    fn test_accept_downloads_serializes_as_protocol_string() {
        for (variant, expected) in [
            (AcceptDownloads::Accept, "accept"),
            (AcceptDownloads::Deny, "deny"),
            (AcceptDownloads::Internal, "internal"),
        ] {
            let options = BrowserContextOptions::builder()
                .accept_downloads(variant)
                .build();
            let value = serde_json::to_value(&options).unwrap();
            assert_eq!(value["acceptDownloads"], serde_json::json!(expected));
        }
    }

    #[test]
    fn test_accept_downloads_bool_compatibility() {
        let opts = BrowserContextOptions::builder()
            .accept_downloads(true)
            .build();
        assert_eq!(opts.accept_downloads, Some(AcceptDownloads::Accept));

        let opts = BrowserContextOptions::builder()
            .accept_downloads(false)
            .build();
        assert_eq!(opts.accept_downloads, Some(AcceptDownloads::Deny));
    }
}
