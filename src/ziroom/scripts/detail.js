() => {
  const clean = value => (value || '').replace(/\s+/g, ' ').trim();
  const absolute = value => { try { return new URL(value, location.href).href; } catch (_) { return null; } };
  const listingId = (location.pathname.match(/\/x\/(\d+)\.html/) || [null, ''])[1];
  const facts = {};
  for (const row of document.querySelectorAll('li,dl,tr')) {
    const key = clean(row.querySelector('dt,th,.label,.name')?.textContent);
    const value = clean(row.querySelector('dd,td,.value')?.textContent);
    if (key && value && key.length < 24 && value.length < 160) facts[key] = value;
  }
  const amenities = [...document.querySelectorAll('.configuration li,.facility li,.amenities li,[class*=config] li')]
    .map(e => clean(e.textContent)).filter(Boolean).filter((value, index, all) => all.indexOf(value) === index);
  const images = [...document.images].map(img => absolute(img.currentSrc || img.src || img.dataset?.src || ''))
    .filter(Boolean).filter(url => /ziroom|static/.test(url));
  const status = [...document.querySelectorAll('span,div')].map(e => clean(e.textContent))
    .find(text => ['可预订','待释放','已出租','空气检测中'].includes(text)) || null;
  const description = clean(document.querySelector('.description,.room-desc,[class*=description]')?.textContent) || null;
  return { listing_id: listingId, status, description, facts, amenities, image_urls: [...new Set(images)] };
}
