// Blog post headings: click a heading to drop its section link into the URL, and
// mirror the section you're reading into the address bar as you scroll. Headings
// stay plain `<hN id>` (not `<a>`), so nothing recolors them — the JS supplies the
// interactivity. curl/no-JS is unaffected; the ids still make shared links jump.

(function () {
  const headings = Array.from(
    document.querySelectorAll(
      '.post-html-content h2[id], .post-html-content h3[id], .post-html-content h4[id]'
    )
  );
  if (!headings.length) return;

  const docTop = (el) => el.getBoundingClientRect().top + window.scrollY;

  // Click a heading → push its slug into the URL and scroll to it.
  headings.forEach((h) => {
    h.addEventListener('click', () => {
      if (location.hash !== '#' + h.id) history.pushState(null, '', '#' + h.id);
      h.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  });

  // Scroll-spy: reflect the current section in the address bar. `replaceState`
  // (not push) so scrolling doesn't spam the back button. The current heading is
  // the last one whose top has crossed a line ~25% down from the viewport top.
  let ticking = false;
  function sync() {
    ticking = false;
    const line = window.scrollY + window.innerHeight * 0.25;
    let current = null;
    for (const h of headings) {
      if (docTop(h) <= line) current = h;
      else break;
    }
    // At the very bottom, force the last section (it may sit below the trigger
    // line and otherwise never become "current").
    if (window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 2) {
      current = headings[headings.length - 1];
    }
    const want = current ? '#' + current.id : '';
    if (want) {
      if (location.hash !== want) history.replaceState(null, '', want);
    } else if (location.hash) {
      history.replaceState(null, '', location.pathname + location.search);
    }
  }
  function onScroll() {
    if (!ticking) {
      ticking = true;
      requestAnimationFrame(sync);
    }
  }
  window.addEventListener('scroll', onScroll, { passive: true });
  sync();
})();
