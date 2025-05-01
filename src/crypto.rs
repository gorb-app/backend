use getrandom::fill;
use hex::encode;

fn generate_access_token() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; 16];
    fill(&mut buf)?;
    Ok(encode(&buf))
}

fn generate_refresh_token() -> Result<String, getrandom::Error> {
    let mut buf = [0u8; 32];
    fill(&mut buf)?;
    Ok(encode(&buf))
}
