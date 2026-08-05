// Password-based encryption for the Ed25519 secret key
// Allows storing the key server-side so any browser can recover it via password

import { hexToBytes, bytesToHex, signBytes, verifyBytes } from './auth.js';
import { x25519 } from '@noble/curves/ed25519.js';

// ── Data encryption key (E2EE) ────────────────────────────────────────────────

export async function deriveDataKey(secretKeyHex) {
  const keyMaterial = await crypto.subtle.importKey(
    'raw', hexToBytes(secretKeyHex), 'HKDF', false, ['deriveKey']
  );
  return crypto.subtle.deriveKey(
    { name: 'HKDF', salt: new Uint8Array(), info: new TextEncoder().encode('questline/sync/v2'), hash: 'SHA-256' },
    keyMaterial, { name: 'AES-GCM', length: 256 }, true, ['encrypt', 'decrypt']
  );
}

export function eventAssociatedData(event) {
  // Mirrors encryption.rs associated_data_for / associated_data_for_route.
  // Account events end at key_id; project events append scope and routing_id.
  const base = `questline-sync-v2|${event.id}|${event.entity_type}|${event.entity_id}|${event.operation}|${event.timestamp}|${event.key_id}`;
  return event.scope === 'project'
    ? `${base}|${event.scope}|${event.routing_id}`
    : base;
}

export function eventSignatureMessage(event) {
  const fields = [event.version, event.id, event.entity_type, event.entity_id,
    event.operation, event.timestamp, event.key_id, event.nonce, event.ciphertext,
    event.scope || 'account', event.routing_id || '', event.device_id || '',
    event.author_public_key];
  return new TextEncoder().encode('questline-sync-event-v1\n' + fields.map(value => {
    const text = String(value);
    return `${new TextEncoder().encode(text).length}:${text}`;
  }).join(''));
}

export async function signEvent(event, identity) {
  event.author_public_key = identity.public_key;
  event.event_signature = await signBytes(identity.secret_key, eventSignatureMessage(event));
  return event;
}

export async function verifyEvent(event) {
  if (!/^[0-9a-f]{64}$/i.test(event.author_public_key || '')
      || !/^[0-9a-f]{128}$/i.test(event.event_signature || '')) return false;
  return verifyBytes(event.author_public_key, event.event_signature, eventSignatureMessage(event));
}

export async function encryptPayload(payloadJson, dataKey, event) {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encoded = new TextEncoder().encode(payloadJson);
  const additionalData = new TextEncoder().encode(eventAssociatedData(event));
  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv, additionalData }, dataKey, encoded);
  return { nonce: bytesToBase64(iv), ciphertext: bytesToBase64(new Uint8Array(ciphertext)) };
}

export async function decryptPayload(event, dataKey) {
  if (!dataKey) throw new Error('Sync encryption key is unavailable');
  if (event.version !== 2
      || (event.key_id !== 'account-v1' && event.key_id !== 'project-v1')
      || !event.nonce || !event.ciphertext) {
    throw new Error('Server returned a plaintext or unsupported sync event');
  }
  const additionalData = new TextEncoder().encode(eventAssociatedData(event));
  const plain = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: base64ToBytes(event.nonce), additionalData },
    dataKey, base64ToBytes(event.ciphertext)
  );
  return new TextDecoder().decode(plain);
}

function bytesToBase64(bytes) { return btoa(String.fromCharCode(...bytes)); }
function base64ToBytes(value) { return Uint8Array.from(atob(value), c => c.charCodeAt(0)); }

// ── Encrypted Fellowship (shared projects) ────────────────────────────────────
// Byte-compatible with src/services/encryption.rs. Interop is locked by the Rust
// test `stable_fellowship_public_key_vector`; do not change domain strings, HKDF
// parameters, or field ordering without updating both sides.

const FELLOWSHIP_IDENTITY_INFO = 'questline/fellowship/x25519/v1';

// HKDF-SHA256 with an empty salt. HMAC zero-pads the key, so an empty salt is
// identical to Rust's `Hkdf::new(None, ..)` (a 32-byte zero salt).
async function hkdfBytes(ikmBytes, infoStr, length = 32) {
  const km = await crypto.subtle.importKey('raw', ikmBytes, 'HKDF', false, ['deriveBits']);
  const bits = await crypto.subtle.deriveBits(
    { name: 'HKDF', salt: new Uint8Array(), info: new TextEncoder().encode(infoStr), hash: 'SHA-256' },
    km, length * 8,
  );
  return new Uint8Array(bits);
}

async function importRawAesKey(keyBytes) {
  return crypto.subtle.importKey('raw', keyBytes, { name: 'AES-GCM' }, false, ['encrypt', 'decrypt']);
}

// A CryptoKey for a raw project key, usable with encryptPayload/decryptPayload.
export async function importProjectKey(keyBytes) {
  return importRawAesKey(keyBytes);
}

// AES-256-GCM under a raw key with raw AAD bytes. Returns base64 nonce/ciphertext
// (ciphertext includes the appended 16-byte GCM tag, matching the aes-gcm crate).
async function aesGcmEncryptRaw(keyBytes, plaintextBytes, aadBytes) {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await importRawAesKey(keyBytes);
  const ct = await crypto.subtle.encrypt({ name: 'AES-GCM', iv, additionalData: aadBytes }, key, plaintextBytes);
  return { nonce: bytesToBase64(iv), ciphertext: bytesToBase64(new Uint8Array(ct)) };
}

async function aesGcmDecryptRaw(keyBytes, nonceB64, ctB64, aadBytes) {
  const key = await importRawAesKey(keyBytes);
  const pt = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: base64ToBytes(nonceB64), additionalData: aadBytes },
    key, base64ToBytes(ctB64),
  );
  return new Uint8Array(pt);
}

// The account's X25519 static secret, HKDF-derived from the Ed25519 identity seed.
async function fellowshipSecretBytes(identity) {
  return hkdfBytes(hexToBytes(identity.secret_key), FELLOWSHIP_IDENTITY_INFO, 32);
}

// The account's X25519 public key (64 hex chars) — registered via `encryption/register`.
export async function fellowshipPublicKeyHex(identity) {
  const secret = await fellowshipSecretBytes(identity);
  return bytesToHex(x25519.getPublicKey(secret));
}

// A fresh random 32-byte project key (used directly as the AES-256-GCM key).
export function generateProjectKey() {
  return crypto.getRandomValues(new Uint8Array(32));
}

// Import a stored project key (hex) back into raw bytes.
export function projectKeyFromHex(hex) { return hexToBytes(hex); }
export function projectKeyToHex(bytes) { return bytesToHex(bytes); }

// Wrap the project key for a recipient: ECDH -> HKDF -> AES-GCM.
// `routingId` is interpolated into both the HKDF info and the GCM AAD.
export async function wrapProjectKey(identity, recipientPubHex, routingId, projectKeyBytes) {
  const secret = await fellowshipSecretBytes(identity);
  const shared = x25519.getSharedSecret(secret, hexToBytes(recipientPubHex));
  const wrappingKey = await hkdfBytes(shared, `questline/fellowship/wrap/v1/${routingId}`, 32);
  return aesGcmEncryptRaw(wrappingKey, projectKeyBytes, new TextEncoder().encode(routingId));
}

// Unwrap a project key delivered by `senderPubHex`. Returns 32 raw bytes.
export async function unwrapProjectKey(identity, senderPubHex, routingId, nonceB64, ctB64) {
  const secret = await fellowshipSecretBytes(identity);
  const shared = x25519.getSharedSecret(secret, hexToBytes(senderPubHex));
  const wrappingKey = await hkdfBytes(shared, `questline/fellowship/wrap/v1/${routingId}`, 32);
  const plain = await aesGcmDecryptRaw(wrappingKey, nonceB64, ctB64, new TextEncoder().encode(routingId));
  if (plain.length !== 32) throw new Error('Unwrapped project key has an invalid length');
  return plain;
}

// Encrypt/decrypt a project payload (invitation name/id, or event content) directly
// under the raw project key with a domain-separated string AAD.
export async function encryptProjectPayload(projectKeyBytes, plaintext, aadStr) {
  return aesGcmEncryptRaw(projectKeyBytes, new TextEncoder().encode(plaintext), new TextEncoder().encode(aadStr));
}

export async function decryptProjectPayload(projectKeyBytes, nonceB64, ctB64, aadStr) {
  const plain = await aesGcmDecryptRaw(projectKeyBytes, nonceB64, ctB64, new TextEncoder().encode(aadStr));
  return new TextDecoder().decode(plain);
}

// Serialize / deserialize the CryptoKey for sessionStorage
export async function exportKeyHex(key) {
  const raw = await crypto.subtle.exportKey('raw', key);
  return bytesToHex(new Uint8Array(raw));
}

export async function importKeyHex(hex) {
  return crypto.subtle.importKey(
    'raw', hexToBytes(hex),
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt']
  );
}

export async function encryptSecretKey(secretKeyHex, password) {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv   = crypto.getRandomValues(new Uint8Array(12));

  const keyMaterial = await crypto.subtle.importKey(
    'raw', new TextEncoder().encode(password), 'PBKDF2', false, ['deriveKey']
  );
  const aesKey = await crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt, iterations: 200_000, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt']
  );

  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    aesKey,
    hexToBytes(secretKeyHex)
  );

  return JSON.stringify({
    salt:       bytesToHex(salt),
    iv:         bytesToHex(iv),
    ciphertext: bytesToHex(new Uint8Array(ciphertext)),
  });
}

export async function decryptSecretKey(blobJson, password) {
  const { salt, iv, ciphertext } = JSON.parse(blobJson);

  const keyMaterial = await crypto.subtle.importKey(
    'raw', new TextEncoder().encode(password), 'PBKDF2', false, ['deriveKey']
  );
  const aesKey = await crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: hexToBytes(salt), iterations: 200_000, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['decrypt']
  );

  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: hexToBytes(iv) },
    aesKey,
    hexToBytes(ciphertext)
  );

  return bytesToHex(new Uint8Array(plaintext));
}
