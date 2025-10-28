// Convert duplicated size-specific pre blocks into a single responsive pre

function pickSize() {
  const w = window.innerWidth || 1024;
  if (w <= 400) return 'tiny';
  if (w <= 600) return 'small';
  if (w <= 900) return 'medium';
  return 'large';
}

function buildResponsivePre(boxEl) {
  const tiny = boxEl.querySelector('.box-tiny pre');
  const small = boxEl.querySelector('.box-small pre');
  const medium = boxEl.querySelector('.box-medium pre');
  const large = boxEl.querySelector('.box-large pre');
  if (!tiny && !small && !medium && !large) return; // nothing to do

  const sanitize = (html) => html || '';
  const map = {
    tiny: tiny ? tiny.innerHTML : (small ? small.innerHTML : (medium ? medium.innerHTML : (large ? large.innerHTML : ''))),
    small: small ? small.innerHTML : (tiny ? tiny.innerHTML : (medium ? medium.innerHTML : (large ? large.innerHTML : ''))),
    medium: medium ? medium.innerHTML : (large ? large.innerHTML : (small ? small.innerHTML : (tiny ? tiny.innerHTML : ''))),
    large: large ? large.innerHTML : (medium ? medium.innerHTML : (small ? small.innerHTML : (tiny ? tiny.innerHTML : '')))
  };

  // Preserve one input container if present (e.g., shoutbox)
  const anyInput = boxEl.querySelector('.shoutbox-input-container');
  const inputClone = anyInput ? anyInput.cloneNode(true) : null;

  // Remove all existing size boxes
  boxEl.querySelectorAll('.box-tiny, .box-small, .box-medium, .box-large').forEach(n => n.remove());

  // Insert the single responsive pre
  const pre = document.createElement('pre');
  pre.className = 'js-responsive-pre';
  pre.dataset.tiny = sanitize(map.tiny);
  pre.dataset.small = sanitize(map.small);
  pre.dataset.medium = sanitize(map.medium);
  pre.dataset.large = sanitize(map.large);
  boxEl.prepend(pre);

  if (inputClone) boxEl.appendChild(inputClone);

  // Initial render
  const sz = pickSize();
  pre.innerHTML = pre.dataset[sz] || '';
}

function updateAll() {
  const sz = pickSize();
  document.querySelectorAll('.js-responsive-pre').forEach(pre => {
    const html = pre.dataset[sz];
    if (html && pre.innerHTML !== html) pre.innerHTML = html;
  });
}

document.addEventListener('DOMContentLoaded', () => {
  // Convert all boxes except protected ones
  document.querySelectorAll('.ascii-box').forEach(box => {
    // Skip specific protected boxes
    if (box.id === 'shoutbox-container') return;
    if (box.hasAttribute('data-no-responsive')) return;
    
    // Convert all others: nav, welcome, posts, categories, footer
    buildResponsivePre(box);
  });
  window.addEventListener('resize', () => {
    clearTimeout(window.__respPreTO);
    window.__respPreTO = setTimeout(updateAll, 100);
  });
});


