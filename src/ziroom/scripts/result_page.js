() => {
  const clean = value => (value || '').replace(/\s+/g, ' ').trim();
  const absolute = value => { try { return new URL(value, location.href).href; } catch (_) { return null; } };
  const detailAnchors = [...document.querySelectorAll('a[href*="/x/"]')]
    .filter((anchor, index, all) => all.findIndex(other => absolute(other.href).replace(/\?.*$/, '') === absolute(anchor.href).replace(/\?.*$/, '')) === index);
  const cards = detailAnchors.map(anchor => {
    const href = absolute(anchor.href);
    const id = (href.match(/\/x\/(\d+)\.html/) || [null, ''])[1];
    let root = anchor;
    for (let depth = 0; root && depth < 8; depth++, root = root.parentElement) {
      if (root.querySelector?.('.price-content') && root.querySelectorAll('a[href*="/x/"]').length <= 3) break;
    }
    if (!root) return null;
    const titleAnchor = [...root.querySelectorAll('a[href*="/x/"]')].find(a => clean(a.textContent));
    const metadata = [...root.querySelectorAll('p,div,span')]
      .map(e => clean(e.textContent)).find(text => text.includes('㎡') && text.includes('|')) || '';
    const locationText = [...root.querySelectorAll('p,div,span')]
      .map(e => clean(e.textContent)).find(text => (text.startsWith('距') || /[\u4e00-\u9fff]+-[\u4e00-\u9fff]+/.test(text)) && text.length < 80) || '';
    const priceRoot = root.querySelector('.price-content');
    const priceGlyphs = priceRoot ? [...priceRoot.querySelectorAll('span.num')].map(span => {
      const style = getComputedStyle(span);
      const rawUrl = style.backgroundImage.match(/url\(["']?(.*?)["']?\)/)?.[1];
      return {
        sprite_url: absolute(rawUrl),
        background_x_px: parseFloat(style.backgroundPositionX || span.style.backgroundPosition || '0'),
        background_size_px: Number.isFinite(parseFloat(style.backgroundSize)) ? parseFloat(style.backgroundSize) : null
      };
    }).filter(glyph => glyph.sprite_url) : [];
    const tagSelectors = '.tag span,.tag a,.tags span,.tip span,.room_tags span,.item-tips span';
    let tags = [...root.querySelectorAll(tagSelectors)].map(e => clean(e.textContent)).filter(Boolean);
    if (!tags.length && priceRoot) {
      tags = [...root.querySelectorAll('span')].map(e => clean(e.textContent))
        .filter(text => text && text.length <= 12 && !text.includes('￥') && !text.includes('㎡'));
    }
    tags = [...new Set(tags)].slice(0, 12);
    const image = root.querySelector('img');
    return {
      listing_id: id, title: clean(titleAnchor?.textContent) || clean(anchor.getAttribute('title')) || id,
      detail_url: href, area_floor_orientation: metadata, location_or_transit: locationText,
      tags, image_url: absolute(image?.currentSrc || image?.src || image?.dataset?.src || ''),
      price_glyphs: priceGlyphs
    };
  }).filter(card => card && card.listing_id && card.detail_url);
  const bodyText = clean(document.body.innerText);
  const totalMatch = bodyText.match(/找到符合条件的\s*(\d+)\s*套/) || bodyText.match(/(\d+)\s*套\s*房源/);
  const next = [...document.querySelectorAll('a')].find(a => clean(a.textContent) === '下一页' && !/(disabled|off)/i.test(`${a.className} ${a.parentElement?.className}`));
  return {
    source_url: location.href, reported_total: totalMatch ? Number(totalMatch[1]) : null,
    cards, next_page_url: next ? absolute(next.href) : null
  };
}
