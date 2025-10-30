// Minimal JS UI wrapper; rendering + WebSocket handled in WASM
import init, { init_shoutbox_client_single, shoutbox_set_expanded, shoutbox_send, shoutbox_reconnect } from '/static/wasm/ascii_web.js';

function setupInputHandlers() {
  const ids = ['shoutbox-input','shoutbox-input-modal'];
  ids.forEach(id => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('keypress', (e) => {
      if (e.key !== 'Enter') return;
      const value = el.value.trim();
      if (!value) return;
      const colon = value.indexOf(':');
      let user = 'anon', msg = value;
      if (colon > 0) { user = value.substring(0, colon).trim(); msg = value.substring(colon + 1).trim(); }
      if (user && msg) shoutbox_send(user, msg);
      el.value = '';
    });
  });
}

function toggleExpand(expand) {
  const modal = document.getElementById('shoutbox-float');
  const base = document.getElementById('shoutbox-container');
  if (!modal || !base) return;
  modal.style.display = expand ? 'block' : 'none';
  base.style.display = expand ? 'none' : '';
  const lines = expand ? (window.innerWidth <= 600 ? 25 : 45) : 5;
  try { shoutbox_set_expanded(!!expand, lines); } catch (_) {}
}

function setupExpandOverlays() {
  const pre = document.getElementById('shoutbox');
  if (!pre) return;
  pre.style.position = 'relative';

  const attach = () => {
    if (pre.querySelector('.sb-icon-hit')) return;
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'sb-icon-hit';
    Object.assign(btn.style, {
      background: 'transparent',
      border: '0',
      padding: '0',
      width: '40px',
      height: '18px',
      right: '6px',
      top: '18px',
      position: 'absolute',
      cursor: 'pointer'
    });
    btn.title = 'Expand shoutbox';
    btn.addEventListener('click', (e) => { e.stopPropagation(); toggleExpand(true); });
    pre.appendChild(btn);
    // Accessibility
    pre.tabIndex = 0;
    pre.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleExpand(true); } });
  };

  // Attach initially and re-attach whenever WASM re-renders and wipes children
  attach();
  const mo = new MutationObserver(() => attach());
  mo.observe(pre, { childList: true, characterData: false, subtree: false });
}

function setupCollapseButton() {
  const btn = document.getElementById('sb-collapse-btn');
  if (!btn) return;
  btn.addEventListener('click', (e) => { e.stopPropagation(); toggleExpand(false); });
}

function setupDrag() {
  const float = document.getElementById('shoutbox-float');
  const handle = document.querySelector('#shoutbox-float .shoutbox-float-content');
  if (!float || !handle) return;
  const enableDrag = () => window.innerWidth > 600;
  if (!enableDrag()) return;
  let dragging = false, offsetX = 0, offsetY = 0;
  handle.addEventListener('mousedown', (e) => {
    dragging = true;
    const rect = float.getBoundingClientRect();
    offsetX = e.clientX - rect.left;
    offsetY = e.clientY - rect.top;
  });
  window.addEventListener('mousemove', (e) => {
    if (!dragging) return;
    float.style.left = `${e.clientX - offsetX}px`;
    float.style.top = `${e.clientY - offsetY}px`;
    float.style.transform = 'none';
  });
  window.addEventListener('mouseup', () => dragging = false);
}

document.addEventListener('DOMContentLoaded', async () => {
  if (window.SADSITE_DEBUG) console.log('shoutbox.js: DOMContentLoaded');
  try {
    if (window.SADSITE_DEBUG) console.log('shoutbox.js: initializing WASM...');
    await init();
    if (window.SADSITE_DEBUG) console.log('shoutbox.js: WASM loaded, calling init_shoutbox_client_single');
    // Single collapsed pre
    init_shoutbox_client_single('shoutbox','shoutbox-modal-pre');
    if (window.SADSITE_DEBUG) console.log('shoutbox.js: shoutbox client initialized');
  } catch (e) {
    if (window.SADSITE_DEBUG) console.error('shoutbox.js: WASM init failed', e);
    // Continue without WASM shoutbox
  }
  setupInputHandlers();
  setupExpandOverlays();
  setupCollapseButton();
  setupDrag();

  // Opportunistic reconnects: online/visibility and periodic
  window.addEventListener('online', () => { try { shoutbox_reconnect(); } catch (_) {} });
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'visible') { try { shoutbox_reconnect(); } catch (_) {} } });
  setInterval(() => { try { shoutbox_reconnect(); } catch (_) {} }, 5000);
});


