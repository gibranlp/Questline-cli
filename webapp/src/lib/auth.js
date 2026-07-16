// Ed25519 auth — mirrors identity.rs and api_client.rs exactly

// PKCS8 header for Ed25519 private key (16 bytes)
// This wraps a raw 32-byte key into the ASN.1 PKCS8 envelope WebCrypto needs
const PKCS8_PREFIX = new Uint8Array([48, 46, 2, 1, 0, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32]);

const STORAGE_KEY = 'questline_identity';

export function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

export function hexToBytes(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}

function buildPkcs8(rawKey32Bytes) {
  const buf = new Uint8Array(PKCS8_PREFIX.length + rawKey32Bytes.length);
  buf.set(PKCS8_PREFIX);
  buf.set(rawKey32Bytes, PKCS8_PREFIX.length);
  return buf.buffer;
}

// Base64 encode — safe for unicode via TextEncoder (matches Rust STANDARD.encode)
export function toBase64(str) {
  const bytes = new TextEncoder().encode(str);
  let binary = '';
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary);
}

export async function generateIdentity() {
  const keyPair = await crypto.subtle.generateKey(
    { name: 'Ed25519' },
    true,
    ['sign', 'verify']
  );

  const pubBytes = await crypto.subtle.exportKey('raw', keyPair.publicKey);
  const publicKeyHex = bytesToHex(new Uint8Array(pubBytes));

  const privPkcs8 = await crypto.subtle.exportKey('pkcs8', keyPair.privateKey);
  // Strip the 16-byte PKCS8 header to get the raw 32-byte key for storage
  const secretKeyHex = bytesToHex(new Uint8Array(privPkcs8).slice(PKCS8_PREFIX.length));

  const identity = {
    user_uuid: crypto.randomUUID(),
    public_key: publicKeyHex,
    secret_key: secretKeyHex,
    device_id: crypto.randomUUID(),
    created_at: new Date().toISOString(),
  };

  localStorage.setItem(STORAGE_KEY, JSON.stringify(identity));
  return identity;
}

export function saveIdentity(identity) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(identity));
}

export function loadIdentity() {
  const raw = localStorage.getItem(STORAGE_KEY);
  return raw ? JSON.parse(raw) : null;
}

export function clearIdentity() {
  localStorage.removeItem(STORAGE_KEY);
  localStorage.removeItem('questline_sync_seq');
  sessionStorage.removeItem('questline_data_key');
}

// sessionStorage holds the raw AES key bytes (hex) for the current browser tab.
// It is cleared when the tab closes, so a fresh login is required per browser session.
export function storeKeyInSession(keyHex) {
  sessionStorage.setItem('questline_data_key', keyHex);
}

export function loadKeyHexFromSession() {
  return sessionStorage.getItem('questline_data_key');
}

export function clearSessionKey() {
  sessionStorage.removeItem('questline_data_key');
}

export async function signRequest(secretKeyHex, timestamp, nonce, bodyToSend) {
  const rawKey = hexToBytes(secretKeyHex);
  const privKey = await crypto.subtle.importKey(
    'pkcs8',
    buildPkcs8(rawKey),
    { name: 'Ed25519' },
    false,
    ['sign']
  );

  // Matches Rust: sign(timestamp + "." + nonce + "." + body_to_send)
  const message = new TextEncoder().encode(`${timestamp}.${nonce}.${bodyToSend}`);
  const sig = await crypto.subtle.sign('Ed25519', privKey, message);
  return bytesToHex(new Uint8Array(sig));
}
