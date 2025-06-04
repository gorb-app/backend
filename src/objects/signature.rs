use std::collections::HashMap;

use actix_web::http::header::HeaderMap;
use url::Url;

use crate::error::Error;

pub struct Signature {
    pub url: Url,
    pub signature: String,
}

impl Signature {
    pub fn from_signature_header(headers: &HeaderMap) -> Result<Signature, Error> {
        let signature_header = headers.get(actix_web::http::header::HeaderName::from_static("signature"));

        if signature_header.is_none() {
            return Err(Error::Unauthorized(
                "No signature header provided".to_string(),
            ));
        }

        let signature_raw = signature_header.unwrap().to_str()?;

        let key_values = signature_raw.split_whitespace();

        let mut hash_map = HashMap::new();

        let results: Result<Vec<()>, Error> = key_values.map(|kv| {
            let mut kv_split = kv.split('=');
            let key = kv_split.next().unwrap().to_string();
            let value = kv_split.next().ok_or(Error::BadRequest(format!(r#"Expected key="value", found {}"#, key)))?.trim_matches('"').to_string();

            hash_map.insert(key, value);

            Ok::<(), Error>(())
        }).collect();

        results?;

        let key_id = hash_map.get("keyId");
        let algorithm = hash_map.get("algorithm");
        let signature = hash_map.get("signature");

        if key_id.is_none() {
            return Err(Error::BadRequest("No keyId was provided".to_string()))
        }


        if algorithm.is_none() {
            return Err(Error::BadRequest("No key algorithm was provided".to_string()))
        }

        if signature.is_none() {
            return Err(Error::BadRequest("No signature was provided".to_string()))
        }

        let key_id = key_id.unwrap();
        let algorithm = algorithm.unwrap();
        let signature = signature.unwrap();
        
        if algorithm != "ed25519" {
            return Err(Error::BadRequest(format!("Unsupported signature {}, please use ed25519", algorithm)))
        }

        Ok(Signature { url: key_id.parse()?, signature: signature.clone() })
    }
}
