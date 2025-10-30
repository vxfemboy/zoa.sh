import init, { AsciiCat } from '/static/wasm/ascii_web.js';

async function runCat() {
  try {
    await init();
    const animations = {};
    const names = [
      'still','alert','yawn','sleep1','sleep2','nrun1','nrun2','nerun1','nerun2',
      'erun1','erun2','serun1','serun2','srun1','srun2','swrun1','swrun2','wrun1','wrun2','nwrun1','nwrun2',
      'nscratch1','nscratch2','escratch1','escratch2','sscratch1','sscratch2','wscratch1','wscratch2','wash','itch1','itch2'
    ];
    for (const name of names) {
      try {
        const resp = await fetch(`/cat/${name}`);
        if (!resp.ok) continue;
        animations[name] = await resp.text();
      } catch (_) {}
    }
    const cat = new AsciiCat(animations);
    document.addEventListener('mousemove', (e) => cat.update_mouse(e.clientX, e.clientY));
    let last;
    function loop(ts) {
      if (!last || ts - last > 30) { last = ts; cat.update(); }
      requestAnimationFrame(loop);
    }
    requestAnimationFrame(loop);
  } catch (e) {
    if (window.SADSITE_DEBUG) console.warn('cat init failed', e);
  }
}

document.addEventListener('DOMContentLoaded', runCat);


