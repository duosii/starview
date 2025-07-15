use std::str::FromStr;

use reqwest::{Client, RequestBuilder, header::HeaderValue};
use starview_common::{
    OptionalBuilder,
    enums::{AssetSize, DeviceType},
};
use tokio::try_join;
use url::Url;
use uuid::Uuid;

use crate::{
    Error, api_url,
    crypto::{decode_base64_msgpack, encode_base64_msgpack, get_request_checksum},
    headers::{Headers, header_name},
    models::{
        ApiResponse, AssetPaths, AssetVersionInfo, GetAssetPathRequest, GetAssetVersionInfoRequest,
        LoadRequest, LoadResponse, SignupRequest, SignupResponse,
    },
};

/// API client that interacts with the game's servers.
pub struct WafuriAPIClient {
    /// The API user's ID.
    pub uuid: String,

    /// The user's ID that was assigned by the server. Not known until login.
    short_uuid: Option<u32>,

    /// The token required to make requests to authenticated endpoints
    login_token: Option<String>,

    /// A session token used by the server
    viewer_id: Option<u32>,

    /// The API client's headers
    headers: Headers,

    /// The reqwest client
    client: Client,

    /// The game server's base API url
    api_host: Url,

    /// The device type that this client will be
    pub device_type: DeviceType,
}

impl WafuriAPIClient {
    pub fn builder() -> WafuriAPIClientBuilder {
        WafuriAPIClientBuilder::new()
    }

    /// Convenience method initializing a [`reqwest::async_impl::request::RequestBuilder`].
    ///
    /// This function will set the RequestBuilder's method to POST, set the URL and body to the provided values, and include headers.
    ///
    /// The request will also be signed.
    fn build_post<U>(&self, url: U, body: String) -> Result<RequestBuilder, Error>
    where
        U: reqwest::IntoUrl,
    {
        let url = url.into_url()?;
        let viewer_id = self.viewer_id.map(|id| id.to_string()).unwrap_or("".into());
        let request_checksum = get_request_checksum(&self.uuid, &viewer_id, url.path(), &body);

        // clone headers and add request checksum to headers
        let mut headers = self.headers.get_cloned_inner();
        headers.insert(
            header_name::PARAM,
            HeaderValue::from_str(&request_checksum)?,
        );

        Ok(self.client.post(url).headers(headers).body(body))
    }

    /// Sets this client's login token
    fn set_login_token(&mut self, login_token: String) -> Result<(), Error> {
        self.headers
            .insert_str(header_name::LOGIN_TOKEN, &login_token)?;
        self.login_token = Some(login_token);
        Ok(())
    }

    /// Sets this client's short uuid
    fn set_short_uuid(&mut self, short_uuid: u32) -> Result<(), Error> {
        self.headers
            .insert_str(header_name::SHORT_UDID, &short_uuid.to_string())?;
        self.short_uuid = Some(short_uuid);
        Ok(())
    }

    /// Signs up with this client's `uuid`.
    ///
    /// If this client is already logged in, this does nothing.
    ///
    /// Returns the SignupResponse from the game server on success.
    pub async fn signup(&mut self) -> Result<Option<SignupResponse>, Error> {
        if self.login_token.is_some() {
            return Ok(None);
        }

        let request = self.build_post(
            self.api_host.join(api_url::TOOL_SIGNUP)?,
            encode_base64_msgpack(&SignupRequest::default())?,
        )?;

        match request.send().await?.error_for_status() {
            Ok(response) => {
                let base64 = response.text().await?;
                let signup_response: ApiResponse<SignupResponse> = decode_base64_msgpack(&base64)?;

                self.set_login_token(signup_response.data.login_token.clone())?;
                self.set_short_uuid(signup_response.data_headers.short_udid)?;
                self.viewer_id = Some(signup_response.data_headers.viewer_id);

                Ok(Some(signup_response.data))
            }
            Err(err) => Err(Error::InvalidRequest(err.to_string())),
        }
    }

    /// Loads the logged in user's data.
    ///
    /// If the client is not logged in, this will return None.
    ///
    /// On success, returns the LoadResponse from the server
    pub async fn load(&self) -> Result<Option<LoadResponse>, Error> {
        if let Some(viewer_id) = self.viewer_id {
            let request = self.build_post(
                self.api_host.join(api_url::LOAD)?,
                encode_base64_msgpack(&LoadRequest::from_viewer_id(viewer_id))?,
            )?;

            match request.send().await?.error_for_status() {
                Ok(response) => {
                    let base64 = response.text().await?;
                    let load_response: ApiResponse<LoadResponse> = decode_base64_msgpack(&base64)?;
                    Ok(Some(load_response.data))
                }
                Err(err) => Err(Error::InvalidRequest(err.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    async fn get_asset_path_device_type(
        &self,
        target_asset_version: &str,
        asset_size: AssetSize,
        device_type: DeviceType,
    ) -> Result<Option<AssetPaths>, Error> {
        if let Some(viewer_id) = self.viewer_id {
            let request = self
                .build_post(
                    self.api_host.join(api_url::ASSET_GET_PATH)?,
                    encode_base64_msgpack(&GetAssetPathRequest::new(
                        target_asset_version.into(),
                        viewer_id,
                    ))?,
                )?
                .header(header_name::ASSET_SIZE, asset_size.to_string())
                .header(header_name::DEVICE, device_type.to_string());

            match request.send().await?.error_for_status() {
                Ok(response) => {
                    let base64 = response.text().await?;
                    let load_response: ApiResponse<AssetPaths> = decode_base64_msgpack(&base64)?;
                    Ok(Some(load_response.data))
                }
                Err(err) => Err(Error::InvalidRequest(err.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    /// Fetches asset paths from the game server
    ///
    /// If the client is not logged in, this will return None
    ///
    /// If this client's device type was set to be `All`, this performs two requests to get the necessary information
    ///
    /// On success, returns the AssetPaths for the provided `target_asset_version` and `asset_size`
    pub async fn get_asset_path(
        &self,
        target_asset_version: &str,
        asset_size: AssetSize,
    ) -> Result<Option<AssetPaths>, Error> {
        match self.device_type {
            DeviceType::Android | DeviceType::Ios => {
                self.get_asset_path_device_type(target_asset_version, asset_size, self.device_type)
                    .await
            }
            DeviceType::All => {
                let android_future = self.get_asset_path_device_type(
                    target_asset_version,
                    asset_size,
                    DeviceType::Android,
                );
                let ios_future = self.get_asset_path_device_type(
                    target_asset_version,
                    asset_size,
                    DeviceType::Ios,
                );

                let (android, ios) = try_join!(android_future, ios_future)?;

                Ok(match (android, ios) {
                    (None, None) => None,
                    (None, Some(ios)) => Some(ios),
                    (Some(android), None) => Some(android),
                    (Some(android), Some(ios)) => Some(android.extend(ios)),
                })
            }
        }
    }

    async fn get_asset_version_info_device_type(
        &self,
        asset_version: &str,
        device_type: DeviceType,
    ) -> Result<Option<AssetVersionInfo>, Error> {
        if let Some(viewer_id) = self.viewer_id {
            let request = self
                .build_post(
                    self.api_host.join(api_url::ASSET_VERSION_INFO)?,
                    encode_base64_msgpack(&GetAssetVersionInfoRequest::new(
                        asset_version.into(),
                        viewer_id,
                    ))?,
                )?
                .header(header_name::DEVICE, device_type.to_string());

            match request.send().await?.error_for_status() {
                Ok(response) => {
                    let base64 = response.text().await?;
                    let load_response: ApiResponse<AssetVersionInfo> =
                        decode_base64_msgpack(&base64)?;
                    Ok(Some(load_response.data))
                }
                Err(err) => Err(Error::InvalidRequest(err.to_string())),
            }
        } else {
            Ok(None)
        }
    }

    /// Gets asset version info from the game server, given an `asset_version`
    pub async fn get_asset_version_info(
        &self,
        asset_version: &str,
    ) -> Result<Vec<AssetVersionInfo>, Error> {
        match self.device_type {
            DeviceType::Android | DeviceType::Ios => {
                if let Some(asset_version_info) = self
                    .get_asset_version_info_device_type(asset_version, self.device_type)
                    .await?
                {
                    Ok(vec![asset_version_info])
                } else {
                    Ok(Vec::new())
                }
            }
            DeviceType::All => {
                let android_future =
                    self.get_asset_version_info_device_type(asset_version, DeviceType::Android);
                let ios_future =
                    self.get_asset_version_info_device_type(asset_version, DeviceType::Ios);

                let (android, ios) = try_join!(android_future, ios_future)?;

                Ok(match (android, ios) {
                    (None, None) => Vec::new(),
                    (None, Some(ios)) => vec![ios],
                    (Some(android), None) => vec![android],
                    (Some(android), Some(ios)) => vec![android, ios],
                })
            }
        }
    }
}

pub struct WafuriAPIClientBuilder {
    uuid: Option<String>,
    short_uuid: Option<u32>,
    login_token: Option<String>,
    viewer_id: Option<u32>,
    api_host: Option<Url>,
    device_type: Option<DeviceType>,
}

impl WafuriAPIClientBuilder {
    pub fn new() -> Self {
        Self {
            uuid: None,
            short_uuid: None,
            login_token: None,
            viewer_id: None,
            api_host: None,
            device_type: None,
        }
    }

    /// Sets this API Client's user ID
    pub fn uuid(mut self, uuid: String) -> Self {
        self.uuid = Some(uuid);
        self
    }

    /// Sets the user ID that was provided by the game server
    pub fn short_uuid(mut self, short_uuid: u32) -> Self {
        self.short_uuid = Some(short_uuid);
        self
    }

    /// Sets the login token that will be used to make authenticated requests
    pub fn login_token(mut self, login_token: String) -> Self {
        self.login_token = Some(login_token);
        self
    }

    /// Sets the session token that will be used by this API client
    pub fn viewer_id(mut self, viewer_id: u32) -> Self {
        self.viewer_id = Some(viewer_id);
        self
    }

    /// The URL that the API client will communicate with
    pub fn api_host(mut self, api_host: Url) -> Self {
        self.api_host = Some(api_host);
        self
    }

    /// Sets the device type that this client will use
    pub fn device_type(mut self, device_type: DeviceType) -> Self {
        self.device_type = Some(device_type);
        self
    }

    /// Attempts to build a WafuriAPIClient
    ///
    /// If a uuid was not provided previously, a random one will be generated
    ///
    /// If an API host was not provided, the default one will be used
    pub fn build(self) -> Result<WafuriAPIClient, Error> {
        let uuid = self
            .uuid
            .unwrap_or_else(|| Uuid::new_v4().to_string().to_uppercase());
        let device_type = self.device_type.unwrap_or(DeviceType::Android);

        let mut api_client = WafuriAPIClient {
            headers: Headers::new(&uuid)?,
            uuid,
            short_uuid: None,
            login_token: None,
            viewer_id: self.viewer_id,
            client: Client::new(),
            api_host: self.api_host.unwrap_or(Url::from_str(api_url::API_HOST)?),
            device_type,
        };

        if let Some(short_uuid) = self.short_uuid {
            api_client.set_short_uuid(short_uuid)?;
        }
        if let Some(login_token) = self.login_token {
            api_client.set_login_token(login_token)?;
        }

        Ok(api_client)
    }
}

impl OptionalBuilder for WafuriAPIClientBuilder {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_builder() {
        let uuid: String = "my-uuid".into();
        let short_uuid: u32 = 218921312;
        let login_token: String = "login-token".into();
        let viewer_id: u32 = 890659012;

        let client = WafuriAPIClient::builder()
            .uuid(uuid.clone())
            .short_uuid(short_uuid)
            .login_token(login_token.clone())
            .viewer_id(viewer_id)
            .build()
            .unwrap();

        assert_eq!(client.uuid, uuid);
        assert_eq!(client.short_uuid, Some(short_uuid));
        assert_eq!(client.login_token, Some(login_token));
        assert_eq!(client.viewer_id, Some(viewer_id));
    }
}
