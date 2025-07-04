use std::path::PathBuf;

use crate::{
    Error,
    download::{DownloadConfig, state::DownloadState},
};
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream};
use reqwest::Client;
use starview_common::fs::write_file;
use tokio::sync::watch;
use tokio_retry::{Action, Retry, strategy::ExponentialBackoff};
use url::Url;

/// Interface for downloading multiple files concurrently
pub struct Downloader {
    state_sender: watch::Sender<DownloadState>,
    config: DownloadConfig,
    client: Client,
}

impl Downloader {
    pub fn new(config: DownloadConfig) -> (Self, watch::Receiver<DownloadState>) {
        let (state_sender, recv) = watch::channel(DownloadState::NotStarted());

        (
            Self {
                state_sender,
                config,
                client: Client::new(),
            },
            recv,
        )
    }

    /// Downloads a file from `url` and saves it to `out_path`
    async fn download_file(client: Client, url: Url, out_path: PathBuf) -> Result<Url, Error> {
        let request = client.get(url.as_str());

        match request.send().await?.error_for_status() {
            Ok(response) => {
                let bytes = response.bytes().await?;
                write_file(&bytes, out_path).await?;
                Ok(url)
            }
            Err(err) => Err(Error::StarviewNet(starview_net::Error::InvalidRequest(
                err.to_string(),
            ))),
        }
    }

    /// Calculates where a file downloaded from a URL should be saved.
    ///
    /// This function removes the host from `url` and appends it onto `out_dir`.
    ///
    /// If `strip_prefix` was provided, that prefix will be stripped from `url` before being appended.
    fn get_url_out_path(url: &Url, out_dir: &PathBuf, strip_prefix: &Option<String>) -> PathBuf {
        let url_path = url.path();

        let stripped_url_path = strip_prefix
            .as_ref()
            .and_then(|remove_prefix| url_path.strip_prefix(remove_prefix))
            .unwrap_or(url_path);

        // make url_path relative
        let relative_url_path = if let Some(stripped) = stripped_url_path.strip_prefix("/") {
            stripped
        } else {
            url_path
        };

        out_dir.join(relative_url_path)
    }

    /// Downloads all urls that were given to this Downloader.
    ///
    /// On success, returns a tuple containing:
    /// - urls that were successfully downloaded
    /// - download errors
    pub async fn download(self) -> Result<(Vec<Url>, Vec<Error>), Error> {
        // generate out_paths
        let to_download_urls: Vec<(Url, PathBuf)> = self
            .config
            .urls
            .into_iter()
            .map(|url| {
                let out_path = Self::get_url_out_path(
                    &url,
                    &self.config.out_path,
                    &self.config.url_strip_prefix,
                );
                (url, out_path)
            })
            .collect();

        // send download start state update
        self.state_sender
            .send_replace(DownloadState::DownloadStart(to_download_urls.len()));

        // download files
        let retry_strategy =
            ExponentialBackoff::from_millis(self.config.retry_delay).take(self.config.retry_count);
        let download_results: Vec<Result<Url, Error>> = stream::iter(to_download_urls)
            .map(|(url, out_path)| {
                let retry_strategy = retry_strategy.clone();
                let client = self.client.clone();
                let state_sender = self.state_sender.clone();
                async move {
                    let download_result = Retry::spawn(
                        retry_strategy,
                        DownloadAction {
                            client,
                            url,
                            out_path,
                        },
                    )
                    .await;

                    // send file download/error state update
                    if download_result.is_ok() {
                        state_sender.send_replace(DownloadState::FileDownload());
                    } else {
                        state_sender.send_replace(DownloadState::DownloadError());
                    }
                    download_result
                }
            })
            .buffer_unordered(self.config.concurrency)
            .collect()
            .await;

        // filter errors out of download_results
        let mut downloaded_urls: Vec<Url> = Vec::new();
        let mut download_errors: Vec<Error> = Vec::new();
        for download_result in download_results {
            match download_result {
                Ok(url) => downloaded_urls.push(url),
                Err(err) => download_errors.push(err),
            }
        }

        // send finish state
        self.state_sender.send_replace(DownloadState::Finish());

        Ok((downloaded_urls, download_errors))
    }
}

struct DownloadAction {
    client: Client,
    url: Url,
    out_path: PathBuf,
}

impl Action for DownloadAction {
    type Future = BoxFuture<'static, Result<Self::Item, Self::Error>>;
    type Item = Url;
    type Error = Error;

    fn run(&mut self) -> Self::Future {
        Downloader::download_file(self.client.clone(), self.url.clone(), self.out_path.clone())
            .boxed()
    }
}
