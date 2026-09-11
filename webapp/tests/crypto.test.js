import test from 'node:test';
import assert from 'node:assert/strict';
import nodeCrypto from 'node:crypto';
import { clearIdentity, saveIdentity } from '../src/lib/auth.js';

import {
  decryptPayload,
  decryptProjectPayload,
  deriveDataKey,
  encryptPayload,
  encryptProjectPayload,
  eventAssociatedData,
  exportKeyHex,
  fellowshipPublicKeyHex,
  generateProjectKey,
  signEvent,
  unwrapProjectKey,
  verifyEvent,
  wrapProjectKey,
} from '../src/lib/crypto.js';

if (!globalThis.crypto) globalThis.crypto = nodeCrypto.webcrypto;

function memoryStorage() {
  const values = new Map();
  return {
    getItem: key => values.has(key) ? values.get(key) : null,
    setItem: (key, value) => values.set(key, String(value)),
    removeItem: key => values.delete(key),
    clear: () => values.clear(),
  };
}

globalThis.localStorage = memoryStorage();
globalThis.sessionStorage = memoryStorage();

function identityFromSeed(byte) {
  const secret = Buffer.alloc(32, byte);
  const prefix = Buffer.from('302e020100300506032b657004220420', 'hex');
  const privateKey = nodeCrypto.createPrivateKey({
    key: Buffer.concat([prefix, secret]),
    format: 'der',
    type: 'pkcs8',
  });
  const publicKey = nodeCrypto.createPublicKey(privateKey)
    .export({ format: 'der', type: 'spki' }).subarray(-32);
  return {
    user_uuid: nodeCrypto.randomUUID(),
    device_id: nodeCrypto.randomUUID(),
    secret_key: secret.toString('hex'),
    public_key: publicKey.toString('hex'),
  };
}

function accountEvent() {
  return {
    version: 2,
    id: nodeCrypto.randomUUID(),
    entity_type: 'note',
    entity_id: nodeCrypto.randomUUID(),
    operation: 'upsert',
    timestamp: '2026-08-01T12:00:00Z',
    key_id: 'account-v1',
    scope: 'account',
    routing_id: '',
    device_id: nodeCrypto.randomUUID(),
  };
}

test('account HKDF stays byte-compatible with Rust', async () => {
  const key = await deriveDataKey('01'.repeat(32));
  assert.equal(
    await exportKeyHex(key),
    '31218b3b3b68d4e9be0f8532c1601d1fd5b54bef07319cdb387904b8daa6cccf',
  );
});

test('account payload encryption authenticates event metadata', async () => {
  const key = await deriveDataKey('01'.repeat(32));
  const event = accountEvent();
  Object.assign(event, await encryptPayload('{"title":"secret"}', key, event));
  assert.equal(await decryptPayload(event, key), '{"title":"secret"}');

  await assert.rejects(
    decryptPayload({ ...event, entity_id: nodeCrypto.randomUUID() }, key),
  );
  assert.match(eventAssociatedData(event), /account-v1$/);
});

test('durable event signatures reject ciphertext and author tampering', async () => {
  const alice = identityFromSeed(1);
  const mallory = identityFromSeed(2);
  const key = await deriveDataKey(alice.secret_key);
  const event = accountEvent();
  Object.assign(event, await encryptPayload('{}', key, event));
  await signEvent(event, alice);

  assert.equal(await verifyEvent(event), true);
  assert.equal(await verifyEvent({ ...event, ciphertext: `${event.ciphertext}A` }), false);
  assert.equal(await verifyEvent({ ...event, author_public_key: mallory.public_key }), false);
});

test('Fellowship public key stays byte-compatible with Rust', async () => {
  assert.equal(
    await fellowshipPublicKeyHex(identityFromSeed(1)),
    '4868ab2c01c594c811d6f5a7e224ad18b0b31b3bb76b0506ff4de4658ed2c429',
  );
});

test('Fellowship envelopes unwrap only for the intended identity and route', async () => {
  const sender = identityFromSeed(1);
  const recipient = identityFromSeed(2);
  const outsider = identityFromSeed(3);
  const route = nodeCrypto.randomUUID();
  const projectKey = generateProjectKey();
  const envelope = await wrapProjectKey(
    sender,
    await fellowshipPublicKeyHex(recipient),
    route,
    projectKey,
  );

  const unwrapped = await unwrapProjectKey(
    recipient,
    await fellowshipPublicKeyHex(sender),
    route,
    envelope.nonce,
    envelope.ciphertext,
  );
  assert.deepEqual(unwrapped, projectKey);
  await assert.rejects(unwrapProjectKey(
    outsider,
    await fellowshipPublicKeyHex(sender),
    route,
    envelope.nonce,
    envelope.ciphertext,
  ));
  await assert.rejects(unwrapProjectKey(
    recipient,
    await fellowshipPublicKeyHex(sender),
    nodeCrypto.randomUUID(),
    envelope.nonce,
    envelope.ciphertext,
  ));
});

test('project payloads fail closed for wrong keys and associated data', async () => {
  const key = generateProjectKey();
  const encrypted = await encryptProjectPayload(key, 'Rivendell', 'name/route-a');
  assert.equal(
    await decryptProjectPayload(key, encrypted.nonce, encrypted.ciphertext, 'name/route-a'),
    'Rivendell',
  );
  await assert.rejects(decryptProjectPayload(
    generateProjectKey(), encrypted.nonce, encrypted.ciphertext, 'name/route-a',
  ));
  await assert.rejects(decryptProjectPayload(
    key, encrypted.nonce, encrypted.ciphertext, 'name/route-b',
  ));
});

test('identity changes clear every account-specific sync cursor', () => {
  const alice = identityFromSeed(1);
  const bob = identityFromSeed(2);
  saveIdentity(alice);
  localStorage.setItem('questline_sync_seq', '10');
  localStorage.setItem('questline_cli_seq', '20');
  localStorage.setItem('questline_project_seq', '30');

  saveIdentity(bob);
  assert.equal(localStorage.getItem('questline_sync_seq'), null);
  assert.equal(localStorage.getItem('questline_cli_seq'), null);
  assert.equal(localStorage.getItem('questline_project_seq'), null);

  localStorage.setItem('questline_project_seq', '40');
  clearIdentity();
  assert.equal(localStorage.getItem('questline_project_seq'), null);
});
