#[derive(Clone, Copy)]
pub enum DownloadState {
    /// The download has not started
    NotStarted(),
    /// the given number of files are being downloaded
    DownloadStart(usize),
    /// A file was downloaded successfully
    FileDownload(),
    /// An error ocurred when downloading a file
    DownloadError(),
    /// The download process completed
    Finish()
}