#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use anyhow::Result;
use futures_util::{Future, StreamExt};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client,
};
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tl::ParserOptions;
use tokio::{
    sync::{mpsc, Barrier},
    time::sleep,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, info};
use url::Url;

const URL_BLACKLIST: [&str; 1] = ["/cdn-cgi/l/email-protection"];

#[derive(Debug, Clone)]
pub struct Config {
    pub delay: Duration,
    pub user_agent: String,
    pub crawling_concurrency: usize,
    pub processing_concurrency: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            crawling_concurrency: 10,
            processing_concurrency: 10,
            delay: Duration::from_millis(5),
            user_agent: "ClippyBot/0.1.0 (clippy.help)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Website {
    base_url: Url,
    config: Config,
    client: Client,
    visited_urls: HashSet<Url>,
}

impl Website {
    /// Creates a new website instance for crawling
    ///
    /// # Errors
    ///
    /// Will throw an error if the base url is invalid
    pub fn new(base_url: &str, config: Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONNECTION, HeaderValue::from_static("keep-alive"));

        let client = Client::builder()
            .gzip(true)
            .brotli(true)
            .default_headers(headers)
            .pool_idle_timeout(None)
            .user_agent(&config.user_agent)
            .tcp_keepalive(Duration::from_millis(500));

        Ok(Self {
            config,
            client: client.build()?,
            visited_urls: HashSet::new(),
            base_url: Url::parse(base_url)?,
        })
    }

    /// Launches the processors that will process the pages and send them to the `on_page` callback
    ///
    /// # Errors
    ///
    /// Will throw an error if sending urls across channels fail.
    pub async fn crawl<F, Fut>(&mut self, on_page: F) -> Result<()>
    where
        F: (Fn(Url, String) -> Fut) + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let crawling_queue_capacity = self.config.crawling_concurrency * 400;
        let processing_queue_capacity = self.config.processing_concurrency * 10;
        let active_spiders = Arc::new(AtomicUsize::new(0));

        let barrier = Arc::new(Barrier::new(3));
        let (pages_tx, pages_rx) = mpsc::channel(processing_queue_capacity);
        let (urls_to_visit_tx, urls_to_visit_rx) = mpsc::channel(crawling_queue_capacity);
        let (new_urls_tx, mut new_urls_rx) = mpsc::channel(crawling_queue_capacity);

        urls_to_visit_tx
            .send(self.base_url.clone())
            .await
            .expect("Failed to send url");

        self.launch_processors(on_page, pages_rx, barrier.clone());

        self.launch_scrapers(
            urls_to_visit_rx,
            new_urls_tx.clone(),
            pages_tx,
            active_spiders.clone(),
            barrier.clone(),
        );

        loop {
            if let Ok((visited_url, new_urls)) = new_urls_rx.try_recv() {
                self.visited_urls.insert(visited_url);

                for url in new_urls {
                    if !self.should_visit(&url) {
                        debug!("Skipping url: {url}");
                        continue;
                    }

                    self.visited_urls.insert(url.clone());
                    urls_to_visit_tx
                        .send(url)
                        .await
                        .expect("Failed to send url");
                }
            }

            if new_urls_tx.capacity() == crawling_queue_capacity
                && urls_to_visit_tx.capacity() == crawling_queue_capacity
                && active_spiders.load(Ordering::SeqCst) == 0
            {
                break;
            }

            sleep(Duration::from_millis(5)).await;
        }

        info!("Finished crawling process");

        drop(urls_to_visit_tx);
        barrier.wait().await;

        Ok(())
    }

    fn launch_processors<F, Fut>(
        &self,
        on_page: F,
        pages_rx: mpsc::Receiver<(Url, String)>,
        barrier: Arc<Barrier>,
    ) where
        F: (Fn(Url, String) -> Fut) + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let concurrency = self.config.processing_concurrency;

        tokio::spawn(async move {
            ReceiverStream::new(pages_rx)
                .for_each_concurrent(concurrency, |(url, html)| async {
                    on_page(url, html).await;
                })
                .await;

            barrier.wait().await;
        });
    }

    fn launch_scrapers(
        &self,
        urls_to_vist: mpsc::Receiver<Url>,
        new_urls: mpsc::Sender<(Url, Vec<Url>)>,
        pages_tx: mpsc::Sender<(Url, String)>,
        active_crawlers: Arc<AtomicUsize>,
        barrier: Arc<Barrier>,
    ) {
        let delay = self.config.delay;
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let concurrency = self.config.crawling_concurrency;

        tokio::spawn(async move {
            ReceiverStream::new(urls_to_vist)
                .for_each_concurrent(concurrency, |queued_url| async {
                    let requested_url = queued_url.clone();
                    debug!("Crawling url: {requested_url}");

                    active_crawlers.fetch_add(1, Ordering::SeqCst);
                    let response = client
                        .get(queued_url)
                        .send()
                        .await
                        .expect("Failed to send request");

                    let url = clean_url(response.url().as_ref(), &base_url);
                    let html = response.text().await.expect("Failed to read response");
                    let urls = find_links(&html, &base_url).expect("Failed to find links");

                    pages_tx
                        .send((url.clone(), html))
                        .await
                        .expect("Failed to send page");

                    new_urls
                        .send((url, urls.into_iter().collect()))
                        .await
                        .expect("Failed to send new urls");

                    debug!("Finished crawling url: {requested_url}");

                    sleep(delay).await;
                    active_crawlers.fetch_sub(1, Ordering::SeqCst);
                })
                .await;

            drop(pages_tx);
            barrier.wait().await;
        });
    }

    fn should_visit(&self, url: &Url) -> bool {
        url.host() == self.base_url.host()
            && url.path().starts_with(self.base_url.path())
            && !self.visited_urls.contains(url)
            && !URL_BLACKLIST.contains(&url.path())
    }
}

fn clean_url(url: &str, base_url: &Url) -> Url {
    let mut url = base_url.join(url).unwrap();

    url.set_query(None);
    url.set_fragment(None);

    url
}

fn find_links(html: &str, base_url: &Url) -> Result<HashSet<Url>> {
    let dom = tl::parse(html, ParserOptions::default())?;
    let parser = dom.parser();
    let mut found_urls = HashSet::new();

    let Some(links) = dom.query_selector("a[href]") else {
        return Ok(found_urls);
    };

    let links = links
        .filter_map(|link| link.get(parser))
        .filter_map(tl::Node::as_tag);

    for link in links {
        let href = link
            .attributes()
            .get("href")
            .flatten()
            .unwrap()
            .as_utf8_str();

        found_urls.insert(clean_url(&href, base_url));
    }

    Ok(found_urls)
}
