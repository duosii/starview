use std::path::{Path, PathBuf};

use url::Url;

/// Configuration options for a Downloader
pub struct DownloadConfig {
    /// In milliseconds, how long between retries.
    ///
    /// This value will increase exponentially every retry
    pub retry_delay: u64,
    pub retry_count: usize,
    pub out_path: PathBuf,
    pub urls: Vec<Url>,
    pub concurrency: usize,
    /// When a downloaded file is saved, this is stripped from the beginning of the out path
    pub url_strip_prefix: Option<String>,
}

impl DownloadConfig {
    /// Creates a new DownloadConfigBuilder
    pub fn builder() -> DownloadConfigBuilder {
        DownloadConfigBuilder::new()
    }
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            retry_delay: 500,
            retry_count: 3,
            out_path: PathBuf::new(),
            urls: Vec::new(),
            concurrency: 5,
            url_strip_prefix: None,
        }
    }
}

pub struct DownloadConfigBuilder {
    config: DownloadConfig,
}

impl DownloadConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: DownloadConfig::default(),
        }
    }

    /// Sets the base time in milliseconds between retries
    pub fn retry_delay(mut self, delay_ms: u64) -> Self {
        self.config.retry_delay = delay_ms;
        self
    }

    /// The maxmimum number of times that a download will be retried
    pub fn retry_count(mut self, retry_count: usize) -> Self {
        self.config.retry_count = retry_count;
        self
    }

    /// Where downloaded files will be saved to
    pub fn out_path(mut self, path: impl AsRef<Path>) -> Self {
        self.config.out_path = path.as_ref().to_path_buf();
        self
    }

    /// The URLs of the files that will be downloaded
    pub fn urls(mut self, urls: Vec<Url>) -> Self {
        self.config.urls = urls;
        self
    }

    /// Sets how many files will be downloaded at a time
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.config.concurrency = concurrency;
        self
    }

    /// Changes where a downloaded file is saved by stripping this from the beginning of the path
    pub fn url_strip_prefix(mut self, strip_prefix: String) -> Self {
        self.config.url_strip_prefix = Some(strip_prefix);
        self
    }

    /// Builds a DownloadConfig from this builder
    pub fn build(self) -> DownloadConfig {
        self.config
    }
}
