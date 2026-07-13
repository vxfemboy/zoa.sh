// Experience timeline: expand/collapse jobs and mirror the open set into the URL
// fragment. Fragment is #exp (focus the block) or #exp;<base36 bitmask of the open
// jobs' data-bit values>. Bits are server-assigned (stable). curl/no-JS shows all.

function expOpenMask() {
  // OR the bits of all open jobs (dedup across desktop/mobile copies).
  let mask = 0;
  document.querySelectorAll('.exp-job.open[data-bit]').forEach((j) => {
    mask |= 1 << Number(j.dataset.bit);
  });
  return mask;
}

function expSetJob(bit, open) {
  document.querySelectorAll('.exp-job[data-bit="' + bit + '"]').forEach((node) => {
    node.classList.toggle('open', open);
    const head = node.querySelector('.exp-head');
    if (head) {
      head.setAttribute('aria-expanded', open ? 'true' : 'false');
      const t = head.querySelector('.exp-toggle');
      if (t) t.textContent = open ? '[-]' : '[+]';
    }
  });
}

function expSyncFragment() {
  const mask = expOpenMask();
  history.replaceState(null, '', mask ? '#exp;' + mask.toString(36) : '#exp');
}

document.querySelectorAll('.exp-head').forEach((head) => {
  head.addEventListener('click', () => {
    const node = head.closest('.exp-job');
    if (!node) return;
    const bit = Number(node.dataset.bit);
    expSetJob(bit, !node.classList.contains('open'));
    expSyncFragment();
  });
});

// Restore from the fragment on load, then scroll to the block.
(function expRestore() {
  const h = location.hash;
  if (!h.startsWith('#exp')) return;
  const semi = h.indexOf(';');
  if (semi !== -1) {
    const mask = parseInt(h.slice(semi + 1), 36) || 0;
    document.querySelectorAll('.exp-job[data-bit]').forEach((j) => {
      if (mask & (1 << Number(j.dataset.bit))) expSetJob(Number(j.dataset.bit), true);
    });
  }
  const target = document.getElementById('exp');
  if (target) target.scrollIntoView({ behavior: 'smooth', block: 'start' });
})();
