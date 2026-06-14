// Experience timeline: click a node to expand/collapse its bio.
// Collapsed by default; curl/no-JS shows everything (bios are in the DOM).
document.querySelectorAll('.exp-head').forEach((head) => {
  head.addEventListener('click', () => {
    const node = head.closest('.exp-job');
    const open = node.classList.toggle('open');
    head.setAttribute('aria-expanded', open ? 'true' : 'false');
    const toggle = head.querySelector('.exp-toggle');
    if (toggle) toggle.textContent = open ? '[-]' : '[+]';
  });
});
