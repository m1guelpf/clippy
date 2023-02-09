use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::env;

type HmacSha256 = Hmac<Sha256>;

pub fn hmac_sha256<T: AsRef<[u8]>>(data: &T) -> Result<String> {
    let app_key = env::var("APP_KEY").context("APP_KEY not found")?;

    Ok(hex::encode(
        HmacSha256::new_from_slice(app_key.as_bytes())?
            .chain_update(data)
            .finalize()
            .into_bytes(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha256_matches_snapshot() {
        env::set_var("APP_KEY", "hunter2");

        assert_eq!(
            hmac_sha256(&"test").unwrap(),
            "4e99265a03bc2001089f7196919be9bbf5b81a557fbb7ea9907a18a461437a04"
        );
    }

    #[test]
    fn fails_when_app_key_not_set() {
        env::remove_var("APP_KEY");
        let err = hmac_sha256(&"test").unwrap_err();

        assert_eq!(err.to_string(), "APP_KEY not found");
    }
}
