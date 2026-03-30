#!/usr/bin/env node
/**
 * Playwright browser sidecar for CodeForge.
 * Communicates via stdin/stdout JSON lines.
 *
 * Commands (send as JSON lines on stdin):
 *   { "cmd": "navigate", "url": "https://..." }
 *   { "cmd": "screenshot" }                        → returns { "type": "screenshot", "data": "<base64 png>" }
 *   { "cmd": "click", "x": 100, "y": 200 }
 *   { "cmd": "type", "text": "hello" }
 *   { "cmd": "scroll", "deltaY": 300 }
 *   { "cmd": "back" }
 *   { "cmd": "forward" }
 *   { "cmd": "reload" }
 *   { "cmd": "extract", "x": 100, "y": 200 }     → returns { "type": "extraction", "html": "...", "css": "..." }
 *   { "cmd": "get_url" }                           → returns { "type": "url", "url": "..." }
 *   { "cmd": "resize", "width": 800, "height": 600 }
 *   { "cmd": "close" }
 */

import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
const __dirname = dirname(fileURLToPath(import.meta.url));
const { chromium } = await import(join(__dirname, '..', 'frontend', 'node_modules', 'playwright', 'index.mjs'));
import { createInterface } from 'readline';

const VIEWPORT = { width: 900, height: 600 };

let browser, context, page;

function send(obj) {
  process.stdout.write(JSON.stringify(obj) + '\n');
}

function sendError(msg) {
  send({ type: 'error', message: msg });
}

async function init() {
  try {
    browser = await chromium.launch({
      headless: true,
      args: ['--disable-blink-features=AutomationControlled'],
    });
    context = await browser.newContext({
      viewport: VIEWPORT,
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
    });
    page = await context.newPage();
    await page.goto('about:blank');
    send({ type: 'ready' });
  } catch (e) {
    sendError(`Failed to launch browser: ${e.message}`);
    process.exit(1);
  }
}

async function handleCommand(line) {
  let cmd;
  try {
    cmd = JSON.parse(line);
  } catch {
    return sendError('Invalid JSON');
  }

  try {
    switch (cmd.cmd) {
      case 'navigate': {
        let url = cmd.url || '';
        if (url && !url.match(/^https?:\/\//)) url = 'https://' + url;
        await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 }).catch(() => {});
        // Wait a bit for rendering
        await page.waitForTimeout(500);
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'navigated', url: page.url() });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;
      }

      case 'screenshot': {
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;
      }

      case 'click': {
        await page.mouse.click(cmd.x || 0, cmd.y || 0);
        await page.waitForTimeout(300);
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;
      }

      case 'type': {
        await page.keyboard.type(cmd.text || '', { delay: 20 });
        await page.waitForTimeout(200);
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;
      }

      case 'keypress': {
        await page.keyboard.press(cmd.key || 'Enter');
        await page.waitForTimeout(300);
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;
      }

      case 'scroll': {
        await page.mouse.wheel(0, cmd.deltaY || 300);
        await page.waitForTimeout(200);
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;
      }

      case 'back':
        await page.goBack({ timeout: 5000 }).catch(() => {});
        await page.waitForTimeout(500);
        send({ type: 'navigated', url: page.url() });
        send({ type: 'screenshot', data: (await page.screenshot({ type: 'png' })).toString('base64') });
        break;

      case 'forward':
        await page.goForward({ timeout: 5000 }).catch(() => {});
        await page.waitForTimeout(500);
        send({ type: 'navigated', url: page.url() });
        send({ type: 'screenshot', data: (await page.screenshot({ type: 'png' })).toString('base64') });
        break;

      case 'reload':
        await page.reload({ timeout: 10000 }).catch(() => {});
        await page.waitForTimeout(500);
        send({ type: 'screenshot', data: (await page.screenshot({ type: 'png' })).toString('base64') });
        break;

      case 'extract': {
        const result = await page.evaluate(({ x, y }) => {
          const el = document.elementFromPoint(x, y);
          if (!el) return { html: '', css: '{}' };

          const html = el.outerHTML.length > 3000
            ? el.outerHTML.substring(0, 3000) + '...'
            : el.outerHTML;

          const computed = window.getComputedStyle(el);
          const keep = ['color','background','background-color','font-size','font-weight','font-family',
            'padding','margin','border','border-radius','display','flex-direction','align-items',
            'justify-content','gap','width','height','max-width','position','box-shadow','text-align',
            'line-height','letter-spacing','overflow'];
          const styles = {};
          const dummy = document.createElement(el.tagName);
          document.body.appendChild(dummy);
          const defaults = window.getComputedStyle(dummy);
          for (const p of keep) {
            const v = computed.getPropertyValue(p);
            const d = defaults.getPropertyValue(p);
            if (v && v !== d && v !== 'none' && v !== 'normal' && v !== 'auto' && v !== '0px')
              styles[p] = v;
          }
          dummy.remove();
          return { html, css: JSON.stringify(styles, null, 2) };
        }, { x: cmd.x || 0, y: cmd.y || 0 });

        send({ type: 'extraction', html: result.html, css: result.css });
        break;
      }

      case 'get_url':
        send({ type: 'url', url: page.url() });
        break;

      case 'resize':
        await page.setViewportSize({
          width: cmd.width || VIEWPORT.width,
          height: cmd.height || VIEWPORT.height,
        });
        const screenshot = await page.screenshot({ type: 'png' });
        send({ type: 'screenshot', data: screenshot.toString('base64') });
        break;

      case 'close':
        await browser.close();
        process.exit(0);

      default:
        sendError(`Unknown command: ${cmd.cmd}`);
    }
  } catch (e) {
    sendError(`Command "${cmd.cmd}" failed: ${e.message}`);
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
