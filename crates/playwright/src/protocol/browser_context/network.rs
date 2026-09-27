use super::{
    BrowserContext, ContextWsRouteHandlerEntry, RouteHandlerEntry, RouteHandlerFuture,
    WsRouteHandlerFuture,
};
use crate::error::{Error, Result};
use crate::protocol::Route;
use crate::protocol::route::UnrouteBehavior;
use crate::server::channel_owner::ChannelOwner;
use std::future::Future;
use std::sync::{Arc, Mutex};

/// Routes, network and WebSocket interception, HAR replay.
impl BrowserContext {
    /// Registers a route handler for context-level network interception.
    ///
    /// Routes registered on a context apply to all pages within the context.
    /// Page-level routes take precedence over context-level routes.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like "**/*.png")
    /// * `handler` - Async closure that handles the route
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-route>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %pattern))]
    pub async fn route<F, Fut>(&self, pattern: &str, handler: F) -> Result<()>
    where
        F: Fn(Route) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler =
            Arc::new(move |route: Route| -> RouteHandlerFuture { Box::pin(handler(route)) });

        self.route_handlers.lock().unwrap().push(RouteHandlerEntry {
            pattern: pattern.to_string(),
            handler,
        });

        self.enable_network_interception().await
    }

    /// Fulfills matching requests from every page in this context using an
    /// in-process tower `Service`, such as an axum `Router` or a tower-http
    /// `ServeDir`, with no socket.
    ///
    /// Each request whose URL matches `pattern` is rebuilt as an
    /// `http::Request`, handed to a clone of `service`, and fulfilled with the
    /// response. The [`route_service`](crate::protocol::route_service) module
    /// documents what the service sees, the limits of route interception
    /// compared with a real listener, and how to wait on a wasm frontend.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to match (supports glob patterns like `"https://app.example/**"`)
    /// * `service` - Any [`RouteService`](crate::protocol::route_service::RouteService):
    ///   an axum `Router`, a tower-http `ServeDir`, a `tower::service_fn`; cloned per request
    ///
    /// # Errors
    ///
    /// Returns an error if network interception cannot be enabled. A service
    /// that fails at request time aborts that request and logs the error; it
    /// does not surface here.
    ///
    /// See: <https://playwright.dev/docs/mock>
    #[cfg(feature = "route-service")]
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %pattern))]
    pub async fn route_service<S: crate::protocol::route_service::RouteService>(
        &self,
        pattern: &str,
        service: S,
    ) -> Result<()> {
        // Captured here, a hop away, so the service's view of the browser
        // (its engine, its cookie jar) does not depend on walking each
        // request's object chain.
        let context = Some(self.clone());
        self.route(pattern, move |route| {
            crate::protocol::route_service::fulfill_from_service(
                route,
                service.clone(),
                context.clone(),
            )
        })
        .await
    }

    /// Removes route handler(s) matching the given URL pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - URL pattern to remove handlers for
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-unroute>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %pattern))]
    pub async fn unroute(&self, pattern: &str) -> Result<()> {
        self.route_handlers
            .lock()
            .unwrap()
            .retain(|entry| entry.pattern != pattern);
        self.enable_network_interception().await
    }

    /// Removes all registered route handlers.
    ///
    /// # Arguments
    ///
    /// * `behavior` - Optional behavior for in-flight handlers
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-unroute-all>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn unroute_all(&self, _behavior: Option<UnrouteBehavior>) -> Result<()> {
        self.route_handlers.lock().unwrap().clear();
        self.enable_network_interception().await
    }

    /// Replays network requests from a HAR file recorded previously.
    ///
    /// Requests matching `options.url` (or all requests if omitted) will be
    /// served from the archive for every page in this context.  Unmatched
    /// requests are either aborted or passed through depending on
    /// `options.not_found` (`"abort"` is the default).
    ///
    /// # Arguments
    ///
    /// * `har_path` - Path to the `.har` file on disk
    /// * `options` - Optional settings (url filter, not_found policy, update mode)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `har_path` does not exist or cannot be read by the Playwright server
    /// - The Playwright server fails to open the archive
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-route-from-har>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn route_from_har(
        &self,
        har_path: &str,
        options: impl Into<Option<crate::protocol::RouteFromHarOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let opts = options.unwrap_or_default();
        let not_found = opts.not_found.unwrap_or_else(|| "abort".to_string());
        let url_filter = opts.url.clone();

        let abs_path = std::path::Path::new(har_path).canonicalize().map_err(|e| {
            Error::InvalidPath(format!(
                "route_from_har: cannot resolve '{}': {}",
                har_path, e
            ))
        })?;
        let abs_str = abs_path.to_string_lossy().into_owned();

        let connection = self.connection();
        let local_utils = {
            let all = connection.all_objects_sync();
            all.into_iter()
                .find(|o| o.type_name() == "LocalUtils")
                .and_then(|o| {
                    o.as_any()
                        .downcast_ref::<crate::protocol::LocalUtils>()
                        .cloned()
                })
                .ok_or_else(|| {
                    Error::ProtocolError(
                        "route_from_har: LocalUtils not found in connection registry".to_string(),
                    )
                })?
        };

        let har_id = local_utils.har_open(&abs_str).await?;

        let pattern = url_filter.unwrap_or_else(|| "**/*".to_string());

        let har_id_clone = har_id.clone();
        let local_utils_clone = local_utils.clone();
        let not_found_clone = not_found.clone();

        self.route(&pattern, move |route| {
            let har_id = har_id_clone.clone();
            let local_utils = local_utils_clone.clone();
            let not_found = not_found_clone.clone();
            async move {
                let request = route.request();
                let req_url = request.url().to_string();
                let req_method = request.method().to_string();

                let headers = crate::protocol::route_params::header_array(request.header_pairs());

                let lookup = local_utils
                    .har_lookup(
                        &har_id,
                        &req_url,
                        &req_method,
                        headers,
                        None,
                        request.is_navigation_request(),
                    )
                    .await;

                match lookup {
                    Err(e) => {
                        tracing::warn!("har_lookup error for {}: {}", req_url, e);
                        route.continue_(None).await
                    }
                    Ok(result) => match result.action.as_str() {
                        "redirect" => {
                            let redirect_url = result.redirect_url.unwrap_or_default();
                            let opts = crate::protocol::ContinueOptions::builder()
                                .url(redirect_url)
                                .build();
                            route.continue_(Some(opts)).await
                        }
                        "fulfill" => {
                            route
                                .fulfill(Some(crate::protocol::route_params::har_fulfill_options(
                                    result.status,
                                    result.body.as_deref(),
                                    result.headers.as_deref(),
                                )))
                                .await
                        }
                        _ => {
                            if not_found == "fallback" {
                                route.fallback(None).await
                            } else {
                                route.abort(None).await
                            }
                        }
                    },
                }
            }
        })
        .await
    }

    /// Intercepts WebSocket connections matching the given URL pattern for all pages in this context.
    ///
    /// When a WebSocket connection from any page in this context matches `url`,
    /// the `handler` is called with a [`WebSocketRoute`](crate::protocol::WebSocketRoute) object.
    /// The handler must call [`connect_to_server`](crate::protocol::WebSocketRoute::connect_to_server)
    /// to forward the connection to the real server, or
    /// [`close`](crate::protocol::WebSocketRoute::close) to terminate it.
    ///
    /// # Arguments
    ///
    /// * `url` — URL glob pattern (e.g. `"ws://**"` or `"wss://example.com/ws"`).
    /// * `handler` — Async closure receiving a `WebSocketRoute`.
    ///
    /// # Errors
    ///
    /// Returns an error if the RPC call to enable interception fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-route-web-socket>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), url = %url))]
    pub async fn route_web_socket<F, Fut>(&self, url: &str, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::WebSocketRoute) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |route: crate::protocol::WebSocketRoute| -> WsRouteHandlerFuture {
                Box::pin(handler(route))
            },
        );

        self.ws_route_handlers
            .lock()
            .unwrap()
            .push(ContextWsRouteHandlerEntry {
                pattern: url.to_string(),
                handler,
            });

        self.enable_ws_interception().await
    }

    /// Updates WebSocket interception patterns for this context.
    async fn enable_ws_interception(&self) -> Result<()> {
        let patterns: Vec<serde_json::Value> = self
            .ws_route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        self.channel()
            .send_no_result(
                "setWebSocketInterceptionPatterns",
                serde_json::json!({ "patterns": patterns }),
            )
            .await
    }

    /// Updates network interception patterns for this context
    async fn enable_network_interception(&self) -> Result<()> {
        let patterns: Vec<serde_json::Value> = self
            .route_handlers
            .lock()
            .unwrap()
            .iter()
            .map(|entry| serde_json::json!({ "glob": entry.pattern }))
            .collect();

        self.channel()
            .send_no_result(
                "setNetworkInterceptionPatterns",
                serde_json::json!({ "patterns": patterns }),
            )
            .await
    }

    /// Handles a route event from the protocol
    pub(super) async fn on_route_event(
        route_handlers: Arc<Mutex<Vec<RouteHandlerEntry>>>,
        route: Route,
    ) {
        let handlers = route_handlers.lock().unwrap().clone();
        let url = route.request().url().to_string();

        for entry in handlers.iter().rev() {
            if crate::protocol::route::matches_pattern(&entry.pattern, &url) {
                let handler = entry.handler.clone();
                if let Err(e) = handler(route.clone()).await {
                    tracing::warn!("Context route handler error: {}", e);
                    // A handler that failed before reaching a route command
                    // leaves the request pending; abort it so the browser
                    // sees a failed request instead of waiting out its timeout.
                    if !route.was_handled()
                        && let Err(abort_error) = route.abort(Some("failed")).await
                    {
                        tracing::warn!("aborting the unhandled route failed too: {}", abort_error);
                    }
                    break;
                }
                if !route.was_handled() {
                    continue;
                }
                break;
            }
        }
    }
}
