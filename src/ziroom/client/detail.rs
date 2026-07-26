use anyhow::Result;
use url::Url;

use super::ZiroomClient;
use crate::{model::ListingDetail, ziroom::trusted_url::TrustedZiroomUrl};

impl ZiroomClient {
    pub(crate) async fn fetch_detail(&self, url: Url) -> Result<ListingDetail> {
        let trusted = TrustedZiroomUrl::parse(url)?;
        let page = self.browser.new_page("about:blank").await?;
        let result = async {
            self.goto_page(&page, trusted).await?;
            self.eval_on(&page, DETAIL_SCRIPT).await
        }
        .await;
        let close_result = page.close().await;
        match (result, close_result) {
            (Ok(detail), Ok(())) => Ok(detail),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
        }
    }
}

const DETAIL_SCRIPT: &str = include_str!("../scripts/detail.js");
