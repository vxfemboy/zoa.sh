// Auto-scrolling image filmstrips for blog posts.
//
// For each `.scroll-container`, duplicate its contents once so the horizontal
// scroll can wrap seamlessly, then drift it left on its own. Hovering pauses the
// drift so you can scroll manually; leaving resumes from where you left off.
// A WASM ASCII scrollbar (◄──███──►) is attached under each strip. Honors
// prefers-reduced-motion.

import initWasm, { AsciiScrollbar, AsciiLightbox } from '/static/wasm/zoa_sh.js';

const SPEED = 0.4; // px per frame

// Keep the WASM structs alive (dropping a JS handle drops the Rust struct).
const scrollbars = [];
const lightboxes = [];

function setupScrollContainer(el) {
  if (el.children.length === 0) return;

  // Duplicate all child nodes (imgs + whitespace) so [half, 2*half] mirrors
  // [0, half] — scrollLeft can then wrap with no visible seam. Clone nodes
  // rather than re-parsing innerHTML (no HTML-string sink).
  const frag = document.createDocumentFragment();
  el.childNodes.forEach((n) => frag.appendChild(n.cloneNode(true)));
  el.appendChild(frag);

  let half = el.scrollWidth / 2;
  const recompute = () => {
    half = el.scrollWidth / 2;
  };
  // scrollWidth isn't final until images have dimensions.
  el.querySelectorAll('img').forEach((img) => {
    if (!img.complete) img.addEventListener('load', recompute);
  });
  window.addEventListener('load', recompute);

  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  let paused = reduce.matches;
  reduce.addEventListener('change', (e) => {
    paused = e.matches;
  });

  el.addEventListener('mouseenter', () => {
    paused = true;
  });
  el.addEventListener('mouseleave', () => {
    // Resume from the user's manual position, normalized into [0, half).
    pos = half > 0 ? el.scrollLeft % half : el.scrollLeft;
    paused = reduce.matches;
  });

  let pos = 0;
  function tick() {
    if (el.dataset.sbDrag) {
      // The WASM ASCII scrollbar owns scrollLeft while its thumb is dragged;
      // follow it so we resume from the dropped position instead of snapping.
      pos = half > 0 ? el.scrollLeft % half : el.scrollLeft;
    } else if (!paused && half > 0) {
      pos += SPEED;
      if (pos >= half) pos -= half;
      el.scrollLeft = pos;
    }
    requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);
}

document.addEventListener('DOMContentLoaded', async () => {
  const containers = document.querySelectorAll('.scroll-container');
  if (containers.length === 0) return;

  // Clone + start the auto-scroll first (this sets the seamless-loop width the
  // WASM scrollbar reads as scrollWidth/2).
  containers.forEach(setupScrollContainer);

  // Attach the WASM ASCII scrollbar. If the module fails to load, the native
  // hover scrollbar remains as a fallback (no has-ascii-scrollbar class added).
  try {
    await initWasm();
    containers.forEach((el) => {
      scrollbars.push(new AsciiScrollbar(el));
      lightboxes.push(new AsciiLightbox(el));
    });
  } catch (e) {
    if (window.ZOA_DEBUG) console.warn('ascii scrollbar/lightbox init failed', e);
  }
});
