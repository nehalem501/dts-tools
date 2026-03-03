use anyhow::Result;
use encoding_rs::WINDOWS_1252;

pub fn get_title(bytes: &[u8]) -> Result<String> {
    let title = match str::from_utf8(&bytes) {
        Ok(t) => Ok(t.to_string()),
        Err(e) => {
            // Invalid UTF-8, let's try decoding it as latin-1
            let (cow, encoding_used, had_errors) = WINDOWS_1252.decode(&bytes);
            if had_errors {
                Err(e)
            } else {
                Ok(cow.as_ref().to_string())
            }
        },
    }?;
    Ok(title)
}