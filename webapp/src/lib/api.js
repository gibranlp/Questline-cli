// API client — mirrors api_client.rs exactly (signing, headers, base64 body)

import { signRequest, toBase64 } from './auth.js';

// Primary API: webapp.questline.com (gibranlp_webappquest DB, fast reads)
const API_BASE = import.meta.env.VITE_API_URL || 'https://questlinecli.com/api/';
// Secondary API: questlinecli.com — used for import pull + dual push to keep CLI in sync
export const QUESTLINE_API_BASE = import.meta.env.VITE_QUESTLINE_API_URL || 'https://questlinecli.com/api/';

export class ApiClient {
  constructor(identity, baseUrl = API_BASE) {
    this.identity = identity;
    this.base     = baseUrl;
  }

  async request(method, route, body = null, queryParams = {}) {
    const timestamp = new Date().toISOString();
    const nonce = crypto.randomUUID();

    // POST bodies are base64-encoded — matches Rust STANDARD.encode(body.as_bytes())
    let bodyToSend = '';
    if (method === 'POST' && body !== null) {
      bodyToSend = toBase64(JSON.stringify(body));
    }

    const signature = await signRequest(
      this.identity.secret_key,
      timestamp,
      nonce,
      bodyToSend
    );

    const url = new URL(this.base);
    url.searchParams.set('route', route);
    for (const [k, v] of Object.entries(queryParams)) {
      url.searchParams.set(k, v);
    }

    const headers = {
      'X-User-Id': this.identity.user_uuid,
      'X-Identity': this.identity.public_key,
      'X-Device-Id': this.identity.device_id,
      'X-Timestamp': timestamp,
      'X-Nonce': nonce,
      'X-Signature': signature,
    };

    if (method === 'POST') {
      headers['Content-Type'] = 'application/json';
    }

    const res = await fetch(url.toString(), {
      method,
      headers,
      body: method === 'POST' ? bodyToSend : undefined,
    });

    if (!res.ok) {
      const text = await res.text();
      let err;
      try { err = JSON.parse(text); } catch { err = { error: text }; }
      throw Object.assign(new Error(err.error || `HTTP ${res.status}`), { status: res.status, body: err });
    }

    return res.json();
  }

  get(route, queryParams = {}) {
    return this.request('GET', route, null, queryParams);
  }

  post(route, body) {
    return this.request('POST', route, body);
  }
}

// Register with access code — creates a webapp account with username/password
// Body is plain JSON (not base64-signed) since this is a public route
export async function registerWithCredentials({ accessCode, username, password, identity, encryptedKeyBlob }) {
  const url = new URL(API_BASE);
  url.searchParams.set('route', 'webapp/register');

  const bodyObj = {
    access_code:        accessCode,
    username:           username,
    password:           password,
    user_id:            identity.user_uuid,
    public_key:         identity.public_key,
    device_id:          identity.device_id,
    device_name:        'Questline Web',
    encrypted_key_blob: encryptedKeyBlob,
  };

  const res = await fetch(url.toString(), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(bodyObj),
  });

  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data; // { status: 'success', user_id: finalUserId }
}

// Login with username/password — returns encrypted key blob for client-side decryption
export async function loginWebapp(username, password) {
  const url = new URL(API_BASE);  // always hits primary (webapp-api proxies to questlinecli.com)
  url.searchParams.set('route', 'webapp/login');

  const res = await fetch(url.toString(), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password }),
  });

  const data = await res.json();
  if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  return data; // { user_id, public_key, encrypted_key_blob }
}

// Pull ALL sync events from questlinecli.com (seq=0) for the initial import.
// Yields batches so the caller can show progress.
export async function* pullAllFromQuestline(identity) {
  const client = new ApiClient(identity, QUESTLINE_API_BASE);
  let seq = 0;
  while (true) {
    const events = await client.request('POST', 'sync/pull', null, { since_seq: seq });
    if (!Array.isArray(events) || events.length === 0) break;
    yield events;
    for (const e of events) { if (e.seq > seq) seq = e.seq; }
    if (events.length < 500) break;
  }
}
