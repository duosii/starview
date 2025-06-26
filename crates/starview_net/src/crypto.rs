use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::{Serialize, de::DeserializeOwned};
use sha1::{Digest, Sha1};

use crate::Error;

/// Encodes a type that implements [`serde::ser::Serialize`], doing the following:
/// 1. Serialize type into msgpack bytes
/// 2. Encode bytes into a base64 string
pub fn encode_base64_msgpack<T: Serialize>(to_encode: &T) -> Result<String, Error> {
    let msgpack_encoded_bytes = rmp_serde::to_vec_named(to_encode)?;
    let base64_encoded = BASE64_STANDARD.encode(msgpack_encoded_bytes);
    Ok(base64_encoded)
}

/// Decodes a string into a type that implements [`serde::de::DeserializeOwned`], doing the following:
/// 1. Decodes the string from base64 into msgpack bytes
/// 2. Deserializes the msgpack bytes into the type
pub fn decode_base64_msgpack<T: DeserializeOwned>(to_decode: &str) -> Result<T, Error> {
    let base64_decoded_bytes = BASE64_STANDARD.decode(&to_decode)?;
    let msgpack_decoded = rmp_serde::from_slice(&base64_decoded_bytes)?;
    Ok(msgpack_decoded)
}

/// Requests to the game server are required to be signed.
///
/// This function generates a checksum that will be accepted by the server.
///
/// The checksum is a SHA1 digest of the request's user ID, api_path, and body.
pub fn get_request_checksum(uuid: &str, viewer_id: &str, api_path: &str, body: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(uuid);
    hasher.update(viewer_id);
    hasher.update(api_path);
    hasher.update(body);

    let digest = hasher.finalize();
    hex::encode(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ExampleStruct {
        name: String,
        level: u8,
    }

    #[test]
    fn test_encode_decode_base64_msgpack() {
        let example_struct = ExampleStruct {
            name: "stella".into(),
            level: 254,
        };
        let encoded = encode_base64_msgpack(&example_struct).unwrap();
        let decoded: ExampleStruct = decode_base64_msgpack(&encoded).unwrap();
        assert_eq!(decoded, example_struct)
    }

    #[test]
    fn test_generate_checksum() {
        let expected = "4749e61694c31600ad5e564bf22b8e3c68d8d26d";
        assert_eq!(
            get_request_checksum(
                "EA5D7426-42A6-474B-26B7-624F5F9B3AF102B3",
                "",
                "/api/index.php/tool/signup",
                "A="
            ),
            expected
        )
    }
}
