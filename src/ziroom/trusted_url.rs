use anyhow::Result;
use url::Url;

use super::BrowserError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TrustedZiroomUrl(Url);

impl TrustedZiroomUrl {
    pub(crate) fn parse(url: Url) -> Result<Self> {
        validate_ziroom_url(&url)?;
        Ok(Self(url))
    }

    pub(crate) fn as_url(&self) -> &Url {
        &self.0
    }
}

pub(crate) fn validate_ziroom_url(url: &Url) -> Result<()> {
    let trusted_host = url.host_str().is_some_and(|host| {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        host == "ziroom.com" || host.ends_with(".ziroom.com")
    });
    if url.scheme() == "https"
        && trusted_host
        && url.username().is_empty()
        && url.password().is_none()
    {
        Ok(())
    } else {
        Err(BrowserError::UnsafeNavigation(redacted_url(url)).into())
    }
}

fn redacted_url(url: &Url) -> String {
    let mut redacted = url.clone();
    redacted.set_query(None);
    redacted.set_fragment(None);
    let _ = redacted.set_username("");
    let _ = redacted.set_password(None);
    redacted.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_ziroom_hosts_only() {
        for value in [
            "https://ziroom.com/z/",
            "https://www.ziroom.com/z/",
            "https://static8.ziroom.com/sprite.png",
        ] {
            assert!(validate_ziroom_url(&Url::parse(value).unwrap()).is_ok());
        }
        for value in [
            "http://www.ziroom.com/z/",
            "https://ziroom.com.example.test/z/",
            "https://user@ziroom.com/z/",
            "file:///tmp/sprite.png",
        ] {
            assert!(validate_ziroom_url(&Url::parse(value).unwrap()).is_err());
        }
    }

    #[test]
    fn rejected_urls_do_not_expose_queries_or_credentials() {
        let url =
            Url::parse("https://user:secret@example.test/path?token=sensitive#fragment").unwrap();
        let error = validate_ziroom_url(&url).unwrap_err().to_string();
        assert!(error.contains("example.test/path"));
        assert!(!error.contains("secret"));
        assert!(!error.contains("sensitive"));
        assert!(!error.contains("fragment"));
    }
}
