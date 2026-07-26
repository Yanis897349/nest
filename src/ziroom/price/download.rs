use anyhow::{Result, bail};
use tokio_util::sync::CancellationToken;

use crate::ziroom::trusted_url::TrustedZiroomUrl;

pub(super) const MAX_SPRITE_BYTES: usize = 1024 * 1024;

pub(super) async fn download_sprite(
    client: &reqwest::Client,
    url: &TrustedZiroomUrl,
    cancel: &CancellationToken,
) -> Result<Vec<u8>> {
    let request = client
        .get(url.as_url().clone())
        .header("Referer", "https://www.ziroom.com/")
        .send();
    let mut response = tokio::select! {
        () = cancel.cancelled() => bail!("price sprite download cancelled"),
        response = request => response?,
    }
    .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_SPRITE_BYTES as u64)
    {
        bail!("price sprite exceeds the {MAX_SPRITE_BYTES}-byte limit");
    }
    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE)
        && !content_type
            .to_str()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .starts_with("image/")
    {
        bail!("price sprite response was not an image");
    }
    let mut bytes = Vec::new();
    loop {
        let chunk = tokio::select! {
            () = cancel.cancelled() => bail!("price sprite download cancelled"),
            chunk = response.chunk() => chunk?,
        };
        let Some(chunk) = chunk else {
            break;
        };
        extend_limited(&mut bytes, &chunk)?;
    }
    Ok(bytes)
}

pub(super) fn extend_limited(buffer: &mut Vec<u8>, chunk: &[u8]) -> Result<()> {
    if buffer.len().saturating_add(chunk.len()) > MAX_SPRITE_BYTES {
        bail!("price sprite exceeds the {MAX_SPRITE_BYTES}-byte limit");
    }
    buffer.extend_from_slice(chunk);
    Ok(())
}
