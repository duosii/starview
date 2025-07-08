#[derive(Clone, Copy, Debug)]
pub enum DownloadState {
    /// The download has not started
    NotStarted(),
    /// the given number of files are being downloaded
    DownloadStart(usize),
    /// A file was downloaded that is the provided number of bytes large
    FileDownload(u64),
    /// An error ocurred when downloading a file
    DownloadError(),
    /// The download process completed
    Finish(),
}
