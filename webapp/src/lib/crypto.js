// Password-based encryption for the Ed25519 secret key
// Allows storing the key server-side so any browser can recover it via password

import { hexToBytes, bytesToHex } from './auth.js';

// ── Data encryption key (E2EE) ────────────────────────────────────────────────

// Derive AES-GCM-256 key from password. Uses userUuid as a deterministic salt
// so the same password always produces the same key for the same account.
export async function deriveDataKey(password, userUuid) {
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    'raw', enc.encode(password), 'PBKDF2', false, ['deriveKey']
  );
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: enc.encode(userUuid), iterations: 200_000, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt']
  );
}

// Encrypt a JSON string → "e2e:<base64(iv + ciphertext)>"
export async function encryptPayload(payloadJson, dataKey) {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encoded = new TextEncoder().encode(payloadJson);
  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, dataKey, encoded);
  const combined = new Uint8Array(12 + ciphertext.byteLength);
  combined.set(iv);
  combined.set(new Uint8Array(ciphertext), 12);
  return 'e2e:' + btoa(String.fromCharCode(...combined));
}

// Decrypt "e2e:<base64>" → plain JSON string.
// Returns the input unchanged if it is not an e2e-prefixed ciphertext.
export async function decryptPayload(content, dataKey) {
  if (!content || !content.startsWith('e2e:')) return content;
  try {
    const combined = Uint8Array.from(atob(content.slice(4)), c => c.charCodeAt(0));
    const iv = combined.slice(0, 12);
    const ciphertext = combined.slice(12);
    const plain = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, dataKey, ciphertext);
    return new TextDecoder().decode(plain);
  } catch {
    return content; // wrong key or corrupted — return raw so the caller can decide
  }
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
