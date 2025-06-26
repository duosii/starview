use reqwest::header::{HeaderMap, HeaderValue, InvalidHeaderValue};

pub mod header_name {
    pub const USER_AGENT: &str = "user-agent";
    pub const CONTENT_TYPE: &str = "content-type";
    pub const PARAM: &str = "param";
    pub const UDID: &str = "udid";
    pub const SHORT_UDID: &str = "short_udid";
    pub const DEVICE_NAME: &str = "device_name";
    pub const APP_VERSION: &str = "app_ver";
    pub const DEVICE: &str = "device";
    pub const FLASH_VERSION: &str = "x-flash-version";
    pub const LOGIN_TOKEN: &str = "login_token";
    pub const ASSET_SIZE: &str = "asset_size";
}

pub mod header_value {
    pub const USER_AGENT: &str =
        "Mozilla/5.0 (Android; U; en-US) AppleWebKit/533.19.4 (KHTML, like Gecko) AdobeAIR/33.1";
    pub const CONTENT_TYPE: &str = "application/x-www-form-urlencoded";
    pub const DEVICE_NAME: &str = "stella";
    pub const APP_VERSION: &str = "1.8.1";
    pub const FLASH_VERSION: &str = "33,1,1,620";
}

/// A collection of headers that the game server expects
#[derive(Default)]
pub struct Headers(pub HeaderMap<HeaderValue>);

impl Headers {
    pub fn new() -> Result<Self, InvalidHeaderValue> {
        let mut headers = Self::default();

        headers.insert_str(header_name::USER_AGENT, header_value::USER_AGENT)?;
        headers.insert_str(header_name::CONTENT_TYPE, header_value::CONTENT_TYPE)?;
        headers.insert_str(header_name::DEVICE_NAME, header_value::DEVICE_NAME)?;
        headers.insert_str(header_name::APP_VERSION, header_value::APP_VERSION)?;
        headers.insert_str(header_name::FLASH_VERSION, header_value::FLASH_VERSION)?;

        Ok(headers)
    }

    pub fn insert(&mut self, name: &'static str, value: HeaderValue) {
        self.0.insert(name, value);
    }

    /// Insert a header with a string value
    pub fn insert_str(
        &mut self,
        name: &'static str,
        value: &str,
    ) -> Result<(), InvalidHeaderValue> {
        self.insert(name, HeaderValue::from_str(value)?);
        Ok(())
    }

    /// Clones the inner HeaderMap and returns it
    pub fn get_cloned_inner(&self) -> HeaderMap {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::headers::{Headers, header_name};

    #[test]
    fn new_headers() {
        assert!(Headers::new().is_ok())
    }

    #[test]
    fn headers_insert_str() {
        let mut headers = Headers::new().unwrap();
        let header_value: &str = "param";
        headers
            .insert_str(header_name::PARAM, header_value)
            .unwrap();

        let inner = headers.get_cloned_inner();
        assert_eq!(inner.get(header_name::PARAM).unwrap(), header_value);
    }
}
