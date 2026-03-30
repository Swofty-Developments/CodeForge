#!/usr/bin/env node
/**
 * CDP Screencast browser sidecar for CodeForge.
 * Launches Chromium via Playwright, streams frames via CDP Page.startScreencast.
 *
 * Stdin/stdout JSON line protocol.
 *
 * Commands:
 *   { "cmd": "navigate", "url": "..." }
 *   { "cmd": "click", "x": 100, "y": 200 }
 *   { "cmd": "scroll", "x": 100, "y": 200, "deltaX": 0, "deltaY": 300 }
 *   { "cmd": "mouseMove", "x": 100, "y": 200 }
 *   { "cmd": "keyDown", "key": "Enter", "text": "" }
 *   { "cmd": "keyUp", "key": "Enter" }
 *   { "cmd": "type", "text": "hello" }
 *   { "cmd": "back" }
 *   { "cmd": "forward" }
 *   { "cmd": "reload" }
 *   { "cmd": "resize", "width": 1280, "height": 720 }
 *   { "cmd": "startInspect" }       → injects element selector overlay
 *   { "cmd": "stopInspect" }        → removes overlay
 *   { "cmd": "extractElement" }     → returns selected element HTML/CSS
 *   { "cmd": "close" }
 *
 * Events emitted:
 *   { "type": "ready" }
 *   { "type": "frame", "data": "<base64 jpeg>" }
 *   { "type": "navigated", "url": "..." }
 *   { "type": "extraction", "html": "...", "css": "..." }
 *   { "type": "error", "message": "..." }
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { createInterface } from 'readline';

const __dirname = dirname(fileURLToPath(import.meta.url));
const { chromium } = await import(join(__dirname, '..', 'frontend', 'node_modules', 'playwright', 'index.mjs'));

let browser, context, page, cdp;
let screencastActive = false;

const VIEWPORT = { width: 1280, height: 800 };

function send(obj) {
  process.stdout.write(JSON.stringify(obj) + '\n');
}

async function init() {
  try {
    browser = await chromium.launch({
      headless: true,
      args: ['--disable-blink-features=AutomationControlled'],
    });
    context = await browser.newContext({
      viewport: VIEWPORT,
      deviceScaleFactor: 2,
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    page = await context.newPage();

    // Open CDP session
    cdp = await page.context().newCDPSession(page);

    // Listen for screencast frames
    cdp.on('Page.screencastFrame', async (params) => {
      send({ type: 'frame', data: params.data });
      // Ack the frame so CDP sends the next one
      try {
        await cdp.send('Page.screencastFrameAck', { sessionId: params.sessionId });
      } catch {}
    });

    // Track navigation
    page.on('framenavigated', (frame) => {
      if (frame === page.mainFrame()) {
        send({ type: 'navigated', url: page.url() });
      }
    });

    await page.goto('about:blank');
    send({ type: 'ready' });
  } catch (e) {
    send({ type: 'error', message: `Launch failed: ${e.message}` });
    process.exit(1);
  }
}

async function startScreencast() {
  if (screencastActive) return;
  screencastActive = true;
  await cdp.send('Page.startScreencast', {
    format: 'jpeg',
    quality: 80,
    maxWidth: VIEWPORT.width * 2,
    maxHeight: VIEWPORT.height * 2,
    everyNthFrame: 1,
  });
}

async function stopScreencast() {
  if (!screencastActive) return;
  screencastActive = false;
  try {
    await cdp.send('Page.stopScreencast');
  } catch {}
}

async function handleCommand(line) {
  let cmd;
  try {
    cmd = JSON.parse(line);
  } catch {
    return send({ type: 'error', message: 'Invalid JSON' });
  }

  try {
    switch (cmd.cmd) {
      case 'navigate': {
        let url = cmd.url || '';
        if (url && !url.match(/^https?:\/\//)) url = 'https://' + url;
        await startScreencast();
        await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 }).catch(() => {});
        send({ type: 'navigated', url: page.url() });
        break;
      }

      case 'click': {
        await cdp.send('Input.dispatchMouseEvent', {
          type: 'mousePressed',
          x: cmd.x,
          y: cmd.y,
          button: 'left',
          clickCount: 1,
        });
        await cdp.send('Input.dispatchMouseEvent', {
          type: 'mouseReleased',
          x: cmd.x,
          y: cmd.y,
          button: 'left',
          clickCount: 1,
        });
        break;
      }

      case 'scroll': {
        await cdp.send('Input.dispatchMouseEvent', {
          type: 'mouseWheel',
          x: cmd.x || VIEWPORT.width / 2,
          y: cmd.y || VIEWPORT.height / 2,
          deltaX: cmd.deltaX || 0,
          deltaY: cmd.deltaY || 0,
        });
        break;
      }

      case 'mouseMove': {
        await cdp.send('Input.dispatchMouseEvent', {
          type: 'mouseMoved',
          x: cmd.x,
          y: cmd.y,
        });
        break;
      }

      case 'keyDown': {
        await cdp.send('Input.dispatchKeyEvent', {
          type: 'keyDown',
          key: cmd.key || '',
          text: cmd.text || '',
          windowsVirtualKeyCode: cmd.keyCode || 0,
        });
        break;
      }

      case 'keyUp': {
        await cdp.send('Input.dispatchKeyEvent', {
          type: 'keyUp',
          key: cmd.key || '',
          windowsVirtualKeyCode: cmd.keyCode || 0,
        });
        break;
      }

      case 'type': {
        for (const char of cmd.text || '') {
          await cdp.send('Input.dispatchKeyEvent', {
            type: 'char',
            text: char,
          });
        }
        break;
      }

      case 'back':
        await page.goBack({ timeout: 5000 }).catch(() => {});
        break;

      case 'forward':
        await page.goForward({ timeout: 5000 }).catch(() => {});
        break;

      case 'reload':
        await page.reload({ timeout: 10000 }).catch(() => {});
        break;

      case 'resize':
        await page.setViewportSize({
          width: cmd.width || VIEWPORT.width,
          height: cmd.height || VIEWPORT.height,
        });
        // Restart screencast with new dimensions
        if (screencastActive) {
          await stopScreencast();
          VIEWPORT.width = cmd.width || VIEWPORT.width;
          VIEWPORT.height = cmd.height || VIEWPORT.height;
          await startScreencast();
        }
        break;

      case 'startInspect': {
        // Inject element inspector overlay
        await page.evaluate(() => {
          if (window.__cfInspector) return;
          window.__cfInspector = true;
          window.__cfSelected = null;

          const ov = document.createElement('div');
          ov.id = 'cf-overlay';
          ov.style.cssText = 'position:fixed;pointer-events:none;z-index:999999;border:2px solid #6b7cff;background:rgba(107,124,255,0.08);display:none;transition:all 60ms;';
          document.body.appendChild(ov);

          const label = document.createElement('div');
          label.id = 'cf-label';
          label.style.cssText = 'position:fixed;z-index:999999;pointer-events:none;background:#6b7cff;color:#fff;font:500 11px/1.4 system-ui;padding:2px 6px;border-radius:3px;display:none;white-space:nowrap;';
          document.body.appendChild(label);

          window.__cfOnMove = (e) => {
            const el = document.elementFromPoint(e.clientX, e.clientY);
            if (!el || el === ov || el === label) return;
            window.__cfHovered = el;
            const r = el.getBoundingClientRect();
            ov.style.display = 'block';
            ov.style.left = r.left + 'px';
            ov.style.top = r.top + 'px';
            ov.style.width = r.width + 'px';
            ov.style.height = r.height + 'px';
            // Show tag name + classes
            let tag = el.tagName.toLowerCase();
            if (el.id) tag += '#' + el.id;
            if (el.className && typeof el.className === 'string')
              tag += '.' + el.className.trim().split(/\s+/).slice(0, 2).join('.');
            label.textContent = tag;
            label.style.display = 'block';
            label.style.left = r.left + 'px';
            label.style.top = Math.max(0, r.top - 22) + 'px';
          };

          window.__cfOnClick = (e) => {
            e.preventDefault();
            e.stopPropagation();
            if (window.__cfHovered) {
              window.__cfSelected = window.__cfHovered;
            }
          };

          document.addEventListener('mousemove', window.__cfOnMove, true);
          document.addEventListener('click', window.__cfOnClick, true);
        });
        break;
      }

      case 'stopInspect': {
        await page.evaluate(() => {
          if (!window.__cfInspector) return;
          document.removeEventListener('mousemove', window.__cfOnMove, true);
          document.removeEventListener('click', window.__cfOnClick, true);
          document.getElementById('cf-overlay')?.remove();
          document.getElementById('cf-label')?.remove();
          window.__cfInspector = false;
        });
        break;
      }

      case 'extractElement': {
        const result = await page.evaluate(() => {
          const el = window.__cfSelected || window.__cfHovered;
          if (!el) return null;
          let html = el.outerHTML;
          if (html.length > 5000) html = html.substring(0, 5000) + '...';

          const cs = getComputedStyle(el);
          const d = document.createElement(el.tagName);
          document.body.appendChild(d);
          const ds = getComputedStyle(d);
          const styles = {};
          const keep = ['color','background','background-color','font-size','font-weight','font-family',
            'padding','margin','border','border-radius','display','flex-direction','align-items',
            'justify-content','gap','width','height','max-width','max-height','position',
            'box-shadow','text-align','line-height','letter-spacing','overflow','opacity',
            'grid-template-columns','grid-template-rows'];
          for (const p of keep) {
            const v = cs.getPropertyValue(p);
            const dv = ds.getPropertyValue(p);
            if (v && v !== dv && v !== 'none' && v !== 'normal' && v !== 'auto' && v !== '0px')
              styles[p] = v;
          }
          d.remove();

          // Get selector path
          let selector = '';
          let current = el;
          while (current && current !== document.body) {
            let s = current.tagName.toLowerCase();
            if (current.id) { s = '#' + current.id; selector = s + (selector ? ' > ' + selector : ''); break; }
            if (current.className && typeof current.className === 'string')
              s += '.' + current.className.trim().split(/\s+/).join('.');
            selector = s + (selector ? ' > ' + selector : '');
            current = current.parentElement;
          }

          return { html, css: JSON.stringify(styles, null, 2), selector };
        });

        if (result) {
          send({ type: 'extraction', html: result.html, css: result.css, selector: result.selector || '' });
        }

        // Clean up inspector
        await page.evaluate(() => {
          document.removeEventListener('mousemove', window.__cfOnMove, true);
          document.removeEventListener('click', window.__cfOnClick, true);
          document.getElementById('cf-overlay')?.remove();
          document.getElementById('cf-label')?.remove();
          window.__cfInspector = false;
          window.__cfSelected = null;
        });
        break;
      }

      case 'close':
        await stopScreencast();
        await browser.close();
        process.exit(0);

      default:
        send({ type: 'error', message: `Unknown command: ${cmd.cmd}` });
    }
  } catch (e) {
    send({ type: 'error', message: `${cmd.cmd}: ${e.message}` });
  }
}

async function main() {
  await init();
  const rl = createInterface({ input: process.stdin });
  rl.on('line', (line) => handleCommand(line));
  rl.on('close', async () => {
    if (browser) await browser.close().catch(() => {});
    process.exit(0);
  });
}

main();
