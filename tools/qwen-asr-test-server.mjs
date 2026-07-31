#!/usr/bin/env node
/**
 * Local Qwen ASR connection tester.
 * Serves tools/qwen-asr-test.html and proxies DashScope calls (avoids browser CORS).
 *
 * Usage:
 *   node tools/qwen-asr-test-server.mjs
 * Then open http://127.0.0.1:8765
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.PORT || 8765);
const HTML_PATH = path.join(__dirname, 'qwen-asr-test.html');

const ALLOWED_HOSTS = new Set([
  'dashscope-intl.aliyuncs.com',
  'dashscope.aliyuncs.com',
]);

function send(res, status, body, type = 'text/plain; charset=utf-8') {
  res.writeHead(status, {
    'Content-Type': type,
    'Cache-Control': 'no-store',
  });
  res.end(body);
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', (c) => chunks.push(c));
    req.on('end', () => {
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'));
      } catch (e) {
        reject(e);
      }
    });
    req.on('error', reject);
  });
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url || '/', `http://${req.headers.host}`);

  if (req.method === 'GET' && (url.pathname === '/' || url.pathname === '/index.html')) {
    const html = fs.readFileSync(HTML_PATH);
    return send(res, 200, html, 'text/html; charset=utf-8');
  }

  if (req.method === 'POST' && url.pathname === '/proxy/qwen') {
    try {
      const payload = await readJson(req);
      const target = String(payload.url || '');
      const apiKey = String(payload.apiKey || '');
      const body = payload.body;

      let host;
      try {
        host = new URL(target).host;
      } catch {
        return send(res, 400, 'Invalid target URL');
      }
      if (!ALLOWED_HOSTS.has(host)) {
        return send(res, 400, `Host not allowed: ${host}`);
      }
      if (!apiKey) {
        return send(res, 400, 'Missing apiKey');
      }

      const upstream = await fetch(target, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${apiKey}`,
          'Content-Type': 'application/json',
          'X-DashScope-SSE': 'disable',
        },
        body: JSON.stringify(body),
      });

      const text = await upstream.text();
      res.writeHead(upstream.status, {
        'Content-Type': upstream.headers.get('content-type') || 'application/json; charset=utf-8',
        'Cache-Control': 'no-store',
      });
      return res.end(text);
    } catch (e) {
      return send(res, 500, `Proxy error: ${e?.message || e}`);
    }
  }

  send(res, 404, 'Not found');
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`Qwen ASR test page: http://127.0.0.1:${PORT}`);
  console.log('Paste your DashScope key, record or pick audio, then Send.');
});
