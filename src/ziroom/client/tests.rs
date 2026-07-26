use std::time::Duration;

use url::Url;

use super::{
    BrowserSettings, ZiroomClient,
    scrape::{PageStart, ScrapeTracker},
};
use crate::ziroom::BrowserError;

#[test]
fn scrape_tracker_stops_loops_limits_pages_and_deduplicates_listings() {
    let first = Url::parse("https://www.ziroom.com/z/").unwrap();
    let mut tracker = ScrapeTracker::default();
    assert_eq!(tracker.begin_page(&first), PageStart::Visit(1));
    assert_eq!(tracker.begin_page(&first), PageStart::Loop);
    assert!(tracker.accept_listing("one"));
    assert!(!tracker.accept_listing("one"));

    let mut tracker = ScrapeTracker::default();
    for page in 1..=100 {
        let url = Url::parse(&format!("https://www.ziroom.com/z/p{page}/")).unwrap();
        assert_eq!(tracker.begin_page(&url), PageStart::Visit(page));
    }
    let extra = Url::parse("https://www.ziroom.com/z/p101/").unwrap();
    assert_eq!(tracker.begin_page(&extra), PageStart::Limit);
}

#[tokio::test]
#[ignore = "opt-in live Ziroom smoke test"]
async fn discovers_live_beijing_filters() {
    assert_eq!(std::env::var("NEST_LIVE_TESTS").as_deref(), Ok("1"));
    let launch = ZiroomClient::launch(BrowserSettings {
        browser_path: std::env::var_os("NEST_BROWSER").map(Into::into),
        headed: std::env::var_os("NEST_LIVE_HEADED").is_some(),
        navigation_timeout: Duration::from_secs(60),
    })
    .await;
    let client = match launch {
        Ok(client) => client,
        Err(error)
            if error
                .downcast_ref::<BrowserError>()
                .is_some_and(|value| matches!(value, BrowserError::ChallengeRequired)) =>
        {
            eprintln!("Ziroom challenge detected; the TUI will retry visibly");
            return;
        }
        Err(error) => panic!("launch browser and load Ziroom: {error:#}"),
    };
    let catalog = client.discover().await.expect("discover filter catalog");
    assert!(catalog.districts.iter().any(|value| value.label == "海淀"));
    assert!(
        catalog
            .rental_types
            .iter()
            .any(|value| value.label == "整租")
    );
    assert!(catalog.bedrooms.iter().any(|value| value.label == "3居+"));
    client.close().await.expect("close browser");
}
