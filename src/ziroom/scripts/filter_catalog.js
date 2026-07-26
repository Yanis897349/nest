() => {
  const clean = value => (value || '').replace(/\s+/g, ' ').trim();
  const direct = element => clean([...element.childNodes]
    .filter(node => node.nodeType === 3).map(node => node.textContent).join(' '));
  const visible = element => !!(element.offsetWidth || element.offsetHeight || element.getClientRects().length);
  const absolute = href => { try { return new URL(href, location.href).href; } catch (_) { return null; } };
  const selected = anchor => {
    const classes = `${anchor.className || ''} ${anchor.parentElement?.className || ''}`.toLowerCase();
    return /(active|selected|current|on)/.test(classes) || anchor.getAttribute('aria-checked') === 'true';
  };
  const option = anchor => ({ label: clean(anchor.textContent), url: absolute(anchor.href), selected: selected(anchor) });
  const ownLabelNodes = label => [...document.querySelectorAll('span,div,dt,li')]
    .filter(element => direct(element) === label);
  const findRow = (label, expected) => {
    for (const title of ownLabelNodes(label)) {
      let node = title.parentElement;
      for (let depth = 0; node && depth < 5; depth++, node = node.parentElement) {
        const labels = [...node.querySelectorAll('a')].map(a => clean(a.textContent));
        if (expected.some(value => labels.includes(value)) && labels.length < 250) return node;
      }
    }
    return null;
  };
  const options = (row, predicate = () => true) => row ? [...row.querySelectorAll('a')]
    .filter(anchor => clean(anchor.textContent) && predicate(anchor))
    .map(option).filter((value, index, all) => all.findIndex(other => other.label === value.label) === index) : [];
  const districtLabels = ['东城','西城','朝阳','海淀','丰台','石景山','通州','昌平','大兴','亦庄开发区','顺义','房山','门头沟','平谷','怀柔','密云','延庆'];
  const regionRow = findRow('区域', districtLabels);
  const subwayRow = findRow('地铁', ['1号线(八通线)','2号线','4号线大兴线','5号线']);
  const commuteInput = [...document.querySelectorAll('input')].find(e => (e.placeholder || '').includes('公司地址'));
  let mode = 'region';
  if (commuteInput && visible(commuteInput)) mode = 'commute';
  else if (subwayRow && visible(subwayRow) && options(subwayRow).length) mode = 'subway';
  const districts = options(regionRow, a => districtLabels.includes(clean(a.textContent)));
  const businessAreas = mode === 'region' ? options(regionRow, a => {
    const label = clean(a.textContent); return !districtLabels.includes(label) && label !== '区域';
  }) : [];
  const subwayLines = options(subwayRow, a => {
    const href = a.getAttribute('href') || ''; const label = clean(a.textContent);
    return /-s[^/]+/.test(href) && !/-t[^/]+/.test(href) && /线/.test(label);
  });
  const stations = options(subwayRow, a => /-t[^/]+/.test(a.getAttribute('href') || ''));
  const typeRow = findRow('类型', ['不限','合租','整租']);
  const rentRow = findRow('租金', ['3000元以下','3000-4000元','13000元以上']);
  const bedroomRow = findRow('户型', ['1居','2居','3居+']);
  const selectedRoot = ownLabelNodes('您已选择:')[0]?.parentElement;
  const selectedLabels = selectedRoot ? [...selectedRoot.querySelectorAll('a')].map(a => clean(a.textContent)).filter(Boolean) : [];
  const sortLabels = ['默认排序','价格','面积'];
  const sortOrders = [...document.querySelectorAll('a')]
    .filter(a => sortLabels.includes(clean(a.textContent))).map(option)
    .filter((value, index, all) => all.findIndex(other => other.label === value.label) === index);
  return {
    mode, districts, business_areas: businessAreas, subway_lines: subwayLines, stations,
    rental_types: options(typeRow), rent_presets: options(rentRow, a => a.tagName === 'A'),
    bedrooms: options(bedroomRow), sort_orders: sortOrders, selected_labels: selectedLabels,
    current_url: location.href
  };
}
