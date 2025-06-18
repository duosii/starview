use std::{path::PathBuf, str::FromStr};

use reqwest::{Client, RequestBuilder, header::HeaderValue};
use url::Url;
use uuid::Uuid;

use crate::{
    api_url, crypto::{decode_base64_msgpack, encode_base64_msgpack, get_request_checksum}, headers::{header_name, Headers}, models::{
        ApiResponse, AssetSize, GetAssetPathRequest, GetAssetPathResponse, GetAssetVersionInfoRequest, GetAssetVersionInfoResponse, LoadRequest, LoadResponse, SignupRequest, SignupResponse
    }, Error
};

/// API client that interacts with the game's servers.
pub struct WafuriAPIClient {
    /// The API user's ID.
    uuid: String,

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
}

impl WafuriAPIClient {
    fn from_uuid_opt_token(uuid: Uuid, login_token: Option<String>) -> Result<Self, Error> {
        let uuid = uuid.to_string().to_uppercase();

        // initialize headers
        let mut headers = Headers::new()?;
        headers.insert_str(header_name::UDID, &uuid)?;

        let mut client = Self {
            uuid,
            short_uuid: None,
            login_token: None,
            viewer_id: None,
            headers,
            client: Client::new(),
            api_host: Url::from_str(api_url::API_HOST)?,
        };

        if let Some(login_token) = login_token {
            client.set_login_token(login_token)?;
        }

        Ok(client)
    }

    /// Creates a new `WafuriAPIClient` using the provided
    /// user ID and login token.
    pub fn from_uuid_login_token(uuid: Uuid, login_token: String) -> Result<Self, Error> {
        Self::from_uuid_opt_token(uuid, Some(login_token))
    }

    /// Creates a new WafuriAPIClient for the provided user ID.
    ///
    /// This client will not be logged in.
    pub fn from_uuid(uuid: Uuid) -> Result<Self, Error> {
        Self::from_uuid_opt_token(uuid, None)
    }

    /// Creates a new WafuriAPIClient with a random user ID.
    ///
    /// This client will not be logged in.
    pub fn new() -> Result<Self, Error> {
        Self::from_uuid(Uuid::new_v4())
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
        let viewer_id = self
            .viewer_id
            .and_then(|id| Some(id.to_string()))
            .unwrap_or("".into());
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

    /// Fetches asset paths from the game server
    ///
    /// If the client is not logged in, this will return None
    ///
    /// On success, returns the AssetPaths for the provided `target_asset_version` and `asset_size`
    pub async fn get_asset_path(
        &self,
        target_asset_version: &str,
        asset_size: AssetSize,
    ) -> Result<Option<GetAssetPathResponse>, Error> {
        if let Some(viewer_id) = self.viewer_id {
            let request = self
                .build_post(
                    self.api_host.join(api_url::ASSET_GET_PATH)?,
                    encode_base64_msgpack(&GetAssetPathRequest::new(
                        target_asset_version.into(),
                        viewer_id,
                    ))?,
                )?
                .header(header_name::ASSET_SIZE, asset_size.to_string());

            match request.send().await?.error_for_status() {
                Ok(response) => {
                    let base64 = response.text().await?;
                    let load_response: ApiResponse<GetAssetPathResponse> =
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
    ) -> Result<Option<GetAssetVersionInfoResponse>, Error> {
        if let Some(viewer_id) = self.viewer_id {
            let request = self.build_post(
                self.api_host.join(api_url::ASSET_VERSION_INFO)?,
                encode_base64_msgpack(&GetAssetVersionInfoRequest::new(asset_version.into(), viewer_id))?,
            )?;

            match request.send().await?.error_for_status() {
                Ok(response) => {
                    let base64 = response.text().await?;
                    let load_response: ApiResponse<GetAssetVersionInfoResponse> =
                        decode_base64_msgpack(&base64)?;
                    Ok(Some(load_response.data))
                }
                Err(err) => Err(Error::InvalidRequest(err.to_string())),
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn example() {
        let mut client = WafuriAPIClient::new().unwrap();
        let signup_response = client.signup().await.unwrap();
        let load_response = client.load().await.unwrap().unwrap();
        let asset_paths_full = client
            .get_asset_path(&load_response.available_asset_version, AssetSize::Full)
            .await
            .unwrap();
        let asset_version_info = client
            .get_asset_version_info(&load_response.available_asset_version)
            .await
            .unwrap();
        println!("{:?}", asset_version_info);
    }
}
