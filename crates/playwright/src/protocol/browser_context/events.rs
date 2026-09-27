use super::{
    BrowserContext, CtxFrameHandler, CtxHandlerFuture, DownloadHandler, PageEventHandler,
    ServiceWorkerHandlerFuture,
};
use crate::error::Result;
use crate::protocol::event_registry::{EventRegistry, Handler};
use crate::protocol::event_waiter::EventWaiter;
use crate::protocol::{Download, Frame, Page, Request, ResponseObject};
use crate::server::channel_owner::ChannelOwner;
use std::future::Future;
use std::sync::{Arc, Mutex};

/// Event subscriptions (`on_*`), page-event forwarders, and one-shot waiters (`expect_*`).
impl BrowserContext {
    /// Adds a listener for the `page` event.
    ///
    /// The handler is called whenever a new page is created in this context,
    /// including popup pages opened through user interactions.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the new `Page`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-page>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_page<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Page> = Arc::new(move |page| Box::pin(handler(page)));
        self.page_events.add_handler(handler);
        Ok(())
    }

    /// Subscribe to `reg`'s event if nothing is listening yet.
    ///
    /// Same contract as `Page::subscribe_if_idle`: the server only pushes an
    /// event once asked, so the first handler or `expect_*` turns it on, and
    /// the name comes from the registry rather than being restated per site.
    async fn subscribe_if_idle<T>(&self, reg: &EventRegistry<T>) {
        if reg.is_idle() {
            _ = self.channel().update_subscription(reg.name(), true).await;
        }
    }

    /// Adds a listener for the `download` event: fired when any page in the
    /// context starts a download. Forwarded from each page's own `download`
    /// event.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-download>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_download<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Download) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |d: Download| -> CtxHandlerFuture { Box::pin(handler(d)) });
        let was_empty = self.download_handlers.lock().unwrap().is_empty();
        self.download_handlers.lock().unwrap().push(handler);
        if was_empty {
            for page in self.pages() {
                Self::wire_download(&page, self.download_handlers.clone()).await;
            }
        }
        Ok(())
    }

    /// Adds a listener for the `frameAttached` event: fired when a frame is
    /// attached in any page of the context. Forwarded from each page.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-frame-attached>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_frame_attached<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |f: Frame| -> CtxHandlerFuture { Box::pin(handler(f)) });
        let was_empty = self.frame_attached_handlers.lock().unwrap().is_empty();
        self.frame_attached_handlers.lock().unwrap().push(handler);
        if was_empty {
            for page in self.pages() {
                Self::wire_frame_attached(&page, self.frame_attached_handlers.clone()).await;
            }
        }
        Ok(())
    }

    /// Adds a listener for the `frameDetached` event: fired when a frame is
    /// detached in any page of the context. Forwarded from each page.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-frame-detached>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_frame_detached<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |f: Frame| -> CtxHandlerFuture { Box::pin(handler(f)) });
        let was_empty = self.frame_detached_handlers.lock().unwrap().is_empty();
        self.frame_detached_handlers.lock().unwrap().push(handler);
        if was_empty {
            for page in self.pages() {
                Self::wire_frame_detached(&page, self.frame_detached_handlers.clone()).await;
            }
        }
        Ok(())
    }

    /// Adds a listener for the `frameNavigated` event: fired when a frame
    /// navigates in any page of the context. Forwarded from each page.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-frame-navigated>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_frame_navigated<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Frame) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |f: Frame| -> CtxHandlerFuture { Box::pin(handler(f)) });
        let was_empty = self.frame_navigated_handlers.lock().unwrap().is_empty();
        self.frame_navigated_handlers.lock().unwrap().push(handler);
        if was_empty {
            for page in self.pages() {
                Self::wire_frame_navigated(&page, self.frame_navigated_handlers.clone()).await;
            }
        }
        Ok(())
    }

    /// Adds a listener for the `pageLoad` event: fired when any page in the
    /// context fires its `load` event. The handler receives that `Page`.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-page-load>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_page_load<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |p: Page| -> CtxHandlerFuture { Box::pin(handler(p)) });
        let was_empty = self.page_load_handlers.lock().unwrap().is_empty();
        self.page_load_handlers.lock().unwrap().push(handler);
        if was_empty {
            for page in self.pages() {
                Self::wire_page_load(&page, self.page_load_handlers.clone()).await;
            }
        }
        Ok(())
    }

    /// Adds a listener for the `pageClose` event: fired when any page in the
    /// context closes. The handler receives that `Page`.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-page-close>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_page_close<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Page) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(move |p: Page| -> CtxHandlerFuture { Box::pin(handler(p)) });
        let was_empty = self.page_close_handlers.lock().unwrap().is_empty();
        self.page_close_handlers.lock().unwrap().push(handler);
        if was_empty {
            for page in self.pages() {
                Self::wire_page_close(&page, self.page_close_handlers.clone()).await;
            }
        }
        Ok(())
    }

    // --- Forwarders: wire a single page's events to the context handler vecs. ---
    // Each (page, event) is wired exactly once: on the first context handler
    // (current pages) or at page creation (future pages, see the "page" event
    // dispatch). The vec is cloned out under the lock before awaiting handlers.

    pub(super) async fn wire_download(page: &Page, handlers: Arc<Mutex<Vec<DownloadHandler>>>) {
        let _ = page
            .on_download(move |d: Download| {
                let handlers = handlers.clone();
                async move {
                    let hs = handlers.lock().unwrap().clone();
                    for h in hs {
                        let _ = h(d.clone()).await;
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub(super) async fn wire_frame_attached(
        page: &Page,
        handlers: Arc<Mutex<Vec<CtxFrameHandler>>>,
    ) {
        let _ = page
            .on_frameattached(move |f: Frame| {
                let handlers = handlers.clone();
                async move {
                    let hs = handlers.lock().unwrap().clone();
                    for h in hs {
                        let _ = h(f.clone()).await;
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub(super) async fn wire_frame_detached(
        page: &Page,
        handlers: Arc<Mutex<Vec<CtxFrameHandler>>>,
    ) {
        let _ = page
            .on_framedetached(move |f: Frame| {
                let handlers = handlers.clone();
                async move {
                    let hs = handlers.lock().unwrap().clone();
                    for h in hs {
                        let _ = h(f.clone()).await;
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub(super) async fn wire_frame_navigated(
        page: &Page,
        handlers: Arc<Mutex<Vec<CtxFrameHandler>>>,
    ) {
        let _ = page
            .on_framenavigated(move |f: Frame| {
                let handlers = handlers.clone();
                async move {
                    let hs = handlers.lock().unwrap().clone();
                    for h in hs {
                        let _ = h(f.clone()).await;
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub(super) async fn wire_page_load(page: &Page, handlers: Arc<Mutex<Vec<PageEventHandler>>>) {
        let p = page.clone();
        let _ = page
            .on_load(move || {
                let handlers = handlers.clone();
                let p = p.clone();
                async move {
                    let hs = handlers.lock().unwrap().clone();
                    for h in hs {
                        let _ = h(p.clone()).await;
                    }
                    Ok(())
                }
            })
            .await;
    }

    pub(super) async fn wire_page_close(page: &Page, handlers: Arc<Mutex<Vec<PageEventHandler>>>) {
        let p = page.clone();
        let _ = page
            .on_close(move || {
                let handlers = handlers.clone();
                let p = p.clone();
                async move {
                    let hs = handlers.lock().unwrap().clone();
                    for h in hs {
                        let _ = h(p.clone()).await;
                    }
                    Ok(())
                }
            })
            .await;
    }

    /// Adds a listener for the `close` event.
    ///
    /// The handler is called when the browser context is closed.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function called with no arguments when the context closes
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-close>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_close<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<()> = Arc::new(move |()| Box::pin(handler()));
        self.close_events.add_handler(handler);
        Ok(())
    }

    /// Adds a listener for the `request` event.
    ///
    /// The handler fires whenever a request is issued from any page in the context.
    /// This is equivalent to subscribing to `on_request` on each individual page,
    /// but covers all current and future pages of the context.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the `Request`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-request>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_request<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Request> = Arc::new(move |request| Box::pin(handler(request)));
        self.subscribe_if_idle(&self.request).await;
        self.request.add_handler(handler);
        Ok(())
    }

    /// Adds a listener for the `requestFinished` event.
    ///
    /// The handler fires after the request has been successfully received by the server
    /// and a response has been fully downloaded for any page in the context.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the completed `Request`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-request-finished>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_request_finished<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Request> = Arc::new(move |request| Box::pin(handler(request)));
        self.subscribe_if_idle(&self.request_finished).await;
        self.request_finished.add_handler(handler);
        Ok(())
    }

    /// Adds a listener for the `requestFailed` event.
    ///
    /// The handler fires when a request from any page in the context fails,
    /// for example due to a network error or if the server returned an error response.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the failed `Request`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-request-failed>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_request_failed<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<Request> = Arc::new(move |request| Box::pin(handler(request)));
        self.subscribe_if_idle(&self.request_failed).await;
        self.request_failed.add_handler(handler);
        Ok(())
    }

    /// Adds a listener for the `response` event.
    ///
    /// The handler fires whenever a response is received from any page in the context.
    ///
    /// Context-level handlers fire before page-level handlers.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the `ResponseObject`
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-response>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_response<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(ResponseObject) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<ResponseObject> =
            Arc::new(move |response| Box::pin(handler(response)));
        self.subscribe_if_idle(&self.response).await;
        self.response.add_handler(handler);
        Ok(())
    }

    /// Adds a listener for the `dialog` event on this browser context.
    ///
    /// The handler fires whenever a JavaScript dialog (alert, confirm, prompt,
    /// or beforeunload) is triggered from **any** page in the context. Context-level
    /// handlers fire before page-level handlers.
    ///
    /// The dialog must be explicitly accepted or dismissed; otherwise the page
    /// will freeze waiting for a response.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async function that receives the [`Dialog`](crate::protocol::Dialog) and calls
    ///   `dialog.accept()` or `dialog.dismiss()`.
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-dialog>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_dialog<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::Dialog> =
            Arc::new(move |dialog| Box::pin(handler(dialog)));
        self.dialog.add_handler(handler);
        Ok(())
    }

    /// Adds a listener for the `dialogclosed` event, which fires once a
    /// dialog has been accepted, dismissed, or closed by the user, on any
    /// page in the context.
    ///
    /// Context-level handlers fire before page-level ones.
    ///
    /// # Errors
    ///
    /// Returns an error if the handler cannot be registered.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-dialog-closed>
    pub async fn on_dialog_closed<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Dialog) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::Dialog> =
            Arc::new(move |dialog| Box::pin(handler(dialog)));
        self.subscribe_if_idle(&self.dialog_closed).await;
        self.dialog_closed.add_handler(handler);
        Ok(())
    }

    /// Subscribe to `dialogClosed` if nothing has yet, so a page-level
    /// handler receives the event even when the context has none of its own.
    pub(crate) async fn ensure_dialog_closed_subscription(&self) {
        self.subscribe_if_idle(&self.dialog_closed).await;
    }

    /// Registers a context-level console event handler.
    ///
    /// The handler fires for any console message emitted by any page in this context.
    /// Context-level handlers fire before page-level handlers.
    ///
    /// The server only sends console events after the first handler is registered
    /// (subscription is managed automatically per context channel).
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives the [`ConsoleMessage`](crate::protocol::ConsoleMessage)
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-console>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_console<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::ConsoleMessage) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::ConsoleMessage> =
            Arc::new(move |msg| Box::pin(handler(msg)));

        self.subscribe_if_idle(&self.console).await;
        self.console.add_handler(handler);

        Ok(())
    }

    /// Registers a context-level handler for uncaught JavaScript exceptions.
    ///
    /// The handler fires whenever a page in this context throws an unhandled
    /// JavaScript error (i.e. an exception that propagates to `window.onerror`
    /// or an unhandled promise rejection). The [`WebError`](crate::protocol::WebError)
    /// passed to the handler contains the error message and an optional back-reference
    /// to the originating page.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure that receives a [`WebError`](crate::protocol::WebError).
    ///
    /// # Errors
    ///
    /// Returns error if communication with the browser process fails.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-web-error>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_weberror<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::WebError) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: Handler<crate::protocol::WebError> =
            Arc::new(move |web_error| Box::pin(handler(web_error)));
        self.weberror.add_handler(handler);
        Ok(())
    }

    /// Registers a handler for the `serviceWorker` event.
    ///
    /// The handler is called when a new service worker is registered in the browser context.
    ///
    /// Note: Service worker testing typically requires HTTPS and a registered service worker.
    ///
    /// # Arguments
    ///
    /// * `handler` - Async closure called with the new [`Worker`](crate::protocol::Worker) object
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-service-worker>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn on_serviceworker<F, Fut>(&self, handler: F) -> Result<()>
    where
        F: Fn(crate::protocol::Worker) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler = Arc::new(
            move |worker: crate::protocol::Worker| -> ServiceWorkerHandlerFuture {
                Box::pin(handler(worker))
            },
        );
        self.serviceworker_handlers.lock().unwrap().push(handler);
        Ok(())
    }

    /// Waits for a new page to be created in this browser context.
    ///
    /// Creates a one-shot waiter that resolves when the next `page` event fires.
    /// The waiter **must** be created before the action that triggers the new page
    /// (e.g. `new_page()` or a user action that opens a popup) to avoid a race
    /// condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if no page is created within the timeout.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// // Set up the waiter BEFORE the triggering action
    /// let waiter = context.expect_page(None).await?;
    /// let _page = context.new_page().await?;
    /// let new_page = waiter.wait().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_page(&self, timeout: Option<f64>) -> Result<EventWaiter<Page>> {
        let rx = self.page_events.wait();
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Waits for this browser context to be closed.
    ///
    /// Creates a one-shot waiter that resolves when the `close` event fires.
    /// The waiter **must** be created before the action that closes the context
    /// to avoid a race condition.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::Timeout`] if the context is not closed within the timeout.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use playwright_rs::Playwright;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pw = Playwright::launch().await?;
    /// # let browser = pw.chromium().launch().await?;
    /// # let context = browser.new_context().await?;
    /// // Set up the waiter BEFORE closing
    /// let waiter = context.expect_close(None).await?;
    /// context.close().await?;
    /// waiter.wait().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_close(&self, timeout: Option<f64>) -> Result<EventWaiter<()>> {
        let rx = self.close_events.wait();
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Waits for a console message from any page in this context.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-event-console>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_console_message(
        &self,
        timeout: Option<f64>,
    ) -> Result<EventWaiter<crate::protocol::ConsoleMessage>> {
        self.subscribe_if_idle(&self.console).await;
        let rx = self.console.wait();
        Ok(EventWaiter::new(rx, timeout.or(Some(30_000.0))))
    }

    /// Waits for the given event to fire and returns a typed `EventValue`.
    ///
    /// This is the generic version of the specific `expect_*` methods. It matches
    /// the playwright-python / playwright-js `context.expect_event(event_name)` API.
    ///
    /// The waiter **must** be created before the action that triggers the event.
    ///
    /// # Supported event names
    ///
    /// `"page"`, `"close"`, `"console"`, `"request"`, `"response"`,
    /// `"weberror"`, `"serviceworker"`
    ///
    /// # Arguments
    ///
    /// * `event` - Event name (case-sensitive, matches Playwright protocol names).
    /// * `timeout` - Timeout in milliseconds. Defaults to 30 000 ms if `None`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::Error::InvalidArgument`] for unknown event names.
    /// Returns [`crate::error::Error::Timeout`] if the event does not fire within the timeout.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-wait-for-event>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn expect_event(
        &self,
        event: &str,
        timeout: Option<f64>,
    ) -> crate::error::Result<EventWaiter<crate::protocol::EventValue>> {
        use crate::protocol::EventValue;
        use tokio::sync::oneshot;

        let timeout_ms = timeout.or(Some(30_000.0));

        match event {
            "page" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.page_events.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(async move {
                    tokio::select! {
                        v = inner_rx => {
                            if let Ok(v) = v { let _ = tx.send(EventValue::Page(v)); }
                        }
                        () = tx.closed() => {}
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "close" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.close_events.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(async move {
                    tokio::select! {
                        v = inner_rx => {
                            if v.is_ok() { let _ = tx.send(EventValue::Close); }
                        }
                        () = tx.closed() => {}
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "console" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.console).await;
                let inner_rx = self.console.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(async move {
                    tokio::select! {
                        v = inner_rx => {
                            if let Ok(v) = v { let _ = tx.send(EventValue::ConsoleMessage(v)); }
                        }
                        () = tx.closed() => {}
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "request" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.request).await;
                let inner_rx = self.request.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(async move {
                    tokio::select! {
                        v = inner_rx => {
                            if let Ok(v) = v { let _ = tx.send(EventValue::Request(v)); }
                        }
                        () = tx.closed() => {}
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "response" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();

                self.subscribe_if_idle(&self.response).await;
                let inner_rx = self.response.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(async move {
                    tokio::select! {
                        v = inner_rx => {
                            if let Ok(v) = v { let _ = tx.send(EventValue::Response(v)); }
                        }
                        () = tx.closed() => {}
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "weberror" => {
                let (mut tx, rx) = oneshot::channel::<EventValue>();
                let inner_rx = self.weberror.wait();

                // select: drop the registry receiver when the caller times
                // out, or a stale FIFO waiter swallows the next event.
                tokio::spawn(async move {
                    tokio::select! {
                        v = inner_rx => {
                            if let Ok(v) = v { let _ = tx.send(EventValue::WebError(v)); }
                        }
                        () = tx.closed() => {}
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            "serviceworker" => {
                let (tx, rx) = oneshot::channel::<EventValue>();
                let (inner_tx, inner_rx) = oneshot::channel::<crate::protocol::Worker>();
                self.serviceworker_waiters.lock().unwrap().push(inner_tx);

                tokio::spawn(async move {
                    if let Ok(v) = inner_rx.await {
                        let _ = tx.send(EventValue::Worker(v));
                    }
                });

                Ok(EventWaiter::new(rx, timeout_ms))
            }

            other => Err(crate::error::Error::InvalidArgument(format!(
                "Unknown event name '{}'. Supported: page, close, console, request, response, \
                 weberror, serviceworker",
                other
            ))),
        }
    }
}
