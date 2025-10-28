document.addEventListener('DOMContentLoaded', function() {
  // Wait for responsive-pre to finish converting boxes
  setTimeout(() => {
    const container = document.querySelector('.latest-posts-box');
    if (!container) { return; }
    const pre = container.querySelector('pre');
    if (!pre) { return; }

    // Only attach if this pre contains a toggle prompt originally
    const hasToggleText = (preText) => (preText || '').includes('▼ SHOW MORE POSTS ▼') || (preText || '').includes('▲ HIDE POSTS ▲');
    if (!hasToggleText(pre.textContent || '')) { return; }

    pre.style.cursor = 'pointer';

    function expand() {
      // Save current (collapsed) HTML so we can restore later
      if (!pre.dataset.collapsedHtml) pre.dataset.collapsedHtml = pre.innerHTML;
      // On mobile, expand to medium; on desktop, expand to large
      const isMobile = (window.innerWidth || 1024) <= 600;
      let expandedHtml = isMobile ? (pre.dataset.medium || pre.dataset.large || pre.innerHTML)
                                  : (pre.dataset.large || pre.dataset.medium || pre.innerHTML);

      // Inject a compact "show less" box inside the outer posts box before the bottom border
      try {
        const lines = expandedHtml.split('\n');
        // Determine inner width from first border line (count of '═')
        const top = lines.find(l => l.startsWith('╔') && l.endsWith('╗')) || '';
        const innerWidth = Math.max(10, (top.match(/═/g) || []).length);
        // Match the size of "▼ SHOW MORE POSTS ▼" which is ~34 chars on mobile, ~56 on desktop
        const subInner = Math.max(28, Math.min(36, innerWidth - 8));
        const label = '▲ HIDE POSTS ▲';
        const padLeft = Math.max(0, Math.floor((subInner - label.length) / 2));
        const padRight = Math.max(0, subInner - label.length - padLeft);
        const subTop = '┌' + '─'.repeat(subInner) + '┐';
        const subMid = '│' + ' '.repeat(padLeft) + label + ' '.repeat(padRight) + '│';
        const subBot = '└' + '─'.repeat(subInner) + '┘';
        // Center the mini-box within the outer box
        const padOuter = Math.max(0, Math.floor((innerWidth - subInner) / 2) - 2);
        const wrappedTop  = '║ ' + ' '.repeat(padOuter) + subTop + ' '.repeat(padOuter) + ' ║';
        const wrappedMid  = '║ ' + ' '.repeat(padOuter) + subMid + ' '.repeat(padOuter) + ' ║';
        const wrappedBot  = '║ ' + ' '.repeat(padOuter) + subBot + ' '.repeat(padOuter) + ' ║';
        // Insert before the last line (bottom border)
        if (lines.length > 2) {
          lines.splice(lines.length - 1, 0, wrappedTop, wrappedMid, wrappedBot);
          expandedHtml = lines.join('\n');
        }
      } catch (_) {}

      pre.innerHTML = expandedHtml;
      pre.dataset.expanded = '1';
    }

    function collapse() {
      const restore = pre.dataset.collapsedHtml;
      if (restore) pre.innerHTML = restore;
      pre.dataset.expanded = '';
    }

    function toggle() {
      // Click target is anywhere in the pre; detect if user clicked inside the mini-box region
      if (pre.dataset.expanded === '1') {
        collapse();
      } else {
        expand();
      }
    }

    pre.addEventListener('click', toggle);
    pre.tabIndex = 0;
    pre.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggle(); } });
  }, 300);
});


