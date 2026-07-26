use anyhow::{Result, bail};
use url::Url;

use super::ZiroomClient;
use crate::model::{FilterCatalog, LocationMode};

impl ZiroomClient {
    pub(crate) async fn discover(&self) -> Result<FilterCatalog> {
        self.eval(FILTER_CATALOG_SCRIPT).await
    }

    pub(crate) async fn navigate_filter(&self, url: Url) -> Result<FilterCatalog> {
        self.goto(url).await?;
        self.discover().await
    }

    pub(crate) async fn set_location_mode(&self, mode: LocationMode) -> Result<FilterCatalog> {
        let label = serde_json::to_string(mode.label())?;
        let script = format!(
            r#"() => {{
                const label = {label};
                const ownText = e => [...e.childNodes].filter(n => n.nodeType === 3)
                    .map(n => n.textContent).join('').trim();
                const candidates = [...document.querySelectorAll('a,div,span')]
                    .filter(e => ownText(e) === label || e.textContent.trim() === label);
                const target = candidates.find(e => e.tagName === 'A') || candidates[0];
                if (!target) return false;
                (target.closest('a') || target).click();
                return true;
            }}"#
        );
        let clicked: bool = self.eval(&script).await?;
        if !clicked {
            bail!("Ziroom location mode {label} was not found");
        }
        self.wait_for_catalog(|catalog| catalog.mode == mode).await
    }

    pub(crate) async fn set_custom_rent(&self, min: u32, max: u32) -> Result<FilterCatalog> {
        if min > max {
            bail!("minimum rent must not exceed maximum rent");
        }
        let previous_url = self.current_url().await?;
        let script = format!(
            r#"() => {{
                const low = document.querySelector('input[name="low"], input.low');
                const high = document.querySelector('input[name="high"], input.high');
                if (!low || !high) return false;
                const set = (el, value) => {{
                    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
                    setter.call(el, String(value));
                    el.dispatchEvent(new Event('input', {{bubbles: true}}));
                    el.dispatchEvent(new Event('change', {{bubbles: true}}));
                }};
                set(low, {min}); set(high, {max});
                const root = low.closest('.price, li, div');
                const button = root && root.querySelector('input.confirm, button, input[type="button"]');
                if (!button) return false;
                button.click(); return true;
            }}"#
        );
        let clicked: bool = self.eval(&script).await?;
        if !clicked {
            bail!("custom rent controls were not found");
        }
        self.wait_for_site().await?;
        self.wait_for_catalog(|catalog| catalog.current_url.as_str() != previous_url)
            .await
    }

    pub(crate) async fn set_commute(&self, address: &str, minutes: u8) -> Result<FilterCatalog> {
        if ![15, 30, 40, 50, 60].contains(&minutes) {
            bail!("commute time must be 15, 30, 40, 50, or 60 minutes");
        }
        self.set_location_mode(LocationMode::Commute).await?;
        let address_json = serde_json::to_string(address)?;
        let type_script = format!(
            r#"() => {{
                const address = {address_json};
                const input = [...document.querySelectorAll('input')]
                    .find(e => (e.placeholder || '').includes('公司地址'));
                if (!input) return false;
                const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
                setter.call(input, address);
                for (const name of ['input', 'change', 'keyup'])
                    input.dispatchEvent(new Event(name, {{bubbles: true}}));
                input.focus(); return true;
            }}"#
        );
        if !self.eval::<bool>(&type_script).await? {
            bail!("commute address input was not found");
        }
        self.wait_for_condition(
            "() => { const input = [...document.querySelectorAll('input')].find(e => (e.placeholder || '').includes('公司地址')); if (!input) return false; const visible = e => !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length); let root = input.parentElement; for (let i = 0; i < 4 && root; i++, root = root.parentElement) { if ([...root.querySelectorAll('li,a')].some(e => visible(e) && e.textContent.trim() && e.textContent.trim() !== input.value)) return true; } return false; }",
            "commute suggestions",
        )
        .await?;
        let previous_url = self.current_url().await?;
        let choose_script = format!(
            r#"() => {{
                const input = [...document.querySelectorAll('input')]
                    .find(e => (e.placeholder || '').includes('公司地址'));
                if (!input) return false;
                const visible = e => !!(e.offsetWidth || e.offsetHeight || e.getClientRects().length);
                let root = input.parentElement;
                for (let i = 0; i < 4 && root; i++, root = root.parentElement) {{
                    const choices = [...root.querySelectorAll('li,a')]
                        .filter(e => visible(e) && e.textContent.trim() && e.textContent.trim() !== input.value);
                    if (choices.length) {{ choices[0].click(); break; }}
                }}
                const time = [...document.querySelectorAll('a')]
                    .find(e => e.textContent.trim() === '{minutes}分');
                if (!time) return false;
                time.click(); return true;
            }}"#
        );
        if !self.eval::<bool>(&choose_script).await? {
            bail!("Ziroom returned no commute suggestion or time control");
        }
        self.wait_for_site().await?;
        self.wait_for_catalog(|catalog| catalog.current_url.as_str() != previous_url)
            .await
    }
}

const FILTER_CATALOG_SCRIPT: &str = include_str!("../scripts/filter_catalog.js");
