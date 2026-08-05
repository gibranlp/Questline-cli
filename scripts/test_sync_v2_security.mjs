#!/usr/bin/env node
import crypto from 'node:crypto';

const baseUrl = process.env.QUESTLINE_API_URL || 'https://questlinecli.com/api/';

function identity() {
  const { publicKey, privateKey } = crypto.generateKeyPairSync('ed25519');
  const publicRaw = publicKey.export({ type: 'spki', format: 'der' }).subarray(-32);
  return { userId: crypto.randomUUID(), deviceId: crypto.randomUUID(), publicKey,
    privateKey, publicHex: publicRaw.toString('hex') };
}

function eventMessage(event) {
  const fields = [event.version, event.id, event.entity_type, event.entity_id,
    event.operation, event.timestamp, event.key_id, event.nonce, event.ciphertext,
    event.scope || 'account', event.routing_id || '', event.device_id || '',
    event.author_public_key];
  return Buffer.from('questline-sync-event-v1\n' + fields.map(value => {
    const text = String(value);
    return `${Buffer.byteLength(text)}:${text}`;
  }).join(''));
}

function makeEvent(who, id = crypto.randomUUID()) {
  const event = { version: 2, id, entity_type: 'note', entity_id: crypto.randomUUID(),
    operation: 'upsert', timestamp: new Date().toISOString(), key_id: 'account-v1',
    nonce: crypto.randomBytes(12).toString('base64'),
    ciphertext: crypto.randomBytes(48).toString('base64'), scope: 'account',
    routing_id: '', device_id: who.deviceId, author_public_key: who.publicHex };
  event.event_signature = crypto.sign(null, eventMessage(event), who.privateKey).toString('hex');
  return event;
}

function makeProjectEvent(who, routingId, entityType = 'note', entityId = null) {
  const event = makeEvent(who);
  event.entity_type = entityType;
  if (entityId) event.entity_id = entityId;
  event.key_id = 'project-v1';
  event.scope = 'project';
  event.routing_id = routingId;
  event.event_signature = crypto.sign(null, eventMessage(event), who.privateKey).toString('hex');
  return event;
}

function signedRequest(who, route, payload, method = 'POST') {
  const body = payload === null ? '' : Buffer.from(JSON.stringify(payload)).toString('base64');
  const timestamp = new Date().toISOString();
  const nonce = crypto.randomUUID();
  const signature = crypto.sign(null, Buffer.from(`${timestamp}.${nonce}.${body}`), who.privateKey).toString('hex');
  return { url: `${baseUrl}?route=${encodeURIComponent(route)}`, body, method, headers: {
    'X-User-Id': who.userId, 'X-Identity': who.publicHex, 'X-Device-Id': who.deviceId,
    'X-Timestamp': timestamp, 'X-Nonce': nonce, 'X-Signature': signature,
    'Content-Type': 'application/json' } };
}

async function send(request) {
  const response = await fetch(request.url, { method: request.method, headers: request.headers,
    body: request.method === 'POST' ? (request.body || undefined) : undefined });
  const text = await response.text();
  let json; try { json = JSON.parse(text); } catch { json = { raw: text }; }
  return { status: response.status, json };
}

function expect(name, result, predicate) {
  if (!predicate(result)) throw new Error(`${name} failed: HTTP ${result.status} ${JSON.stringify(result.json)}`);
  console.log(`PASS ${name}: HTTP ${result.status}`);
}

const alice = identity();
const mallory = identity();

const registration = await send(signedRequest(alice, 'sync/v2/pull', null));
expect('ephemeral identity registration', registration, r => r.status === 200);
await send(signedRequest(mallory, 'sync/v2/pull', null));

const validEvent = makeEvent(alice);
const firstRequest = signedRequest(alice, 'sync/v2/push', [validEvent]);
expect('valid signed event', await send(firstRequest), r => r.status === 200 && r.json.pushed === 1);
expect('request nonce replay rejected', await send(firstRequest), r => r.status === 401 && /Replay/.test(r.json.error || ''));

const substituted = { ...makeEvent(alice), ciphertext: crypto.randomBytes(48).toString('base64') };
expect('ciphertext substitution rejected', await send(signedRequest(alice, 'sync/v2/push', [substituted])), r => r.status === 400);

expect('cross-identity authorship rejected', await send(signedRequest(mallory, 'sync/v2/push', [makeEvent(alice)])), r => r.status === 400);

expect('duplicate event ID ignored', await send(signedRequest(alice, 'sync/v2/push', [validEvent])), r => r.status === 200 && r.json.pushed === 0);

const snapshotEvent = makeEvent(alice);
expect('signed snapshot cutover', await send(signedRequest(alice, 'sync/v2/snapshot', [snapshotEvent])), r => r.status === 200);
const unsigned = makeEvent(alice);
delete unsigned.author_public_key;
delete unsigned.event_signature;
expect('unsigned post-cutover write rejected', await send(signedRequest(alice, 'sync/v2/push', [unsigned])), r => r.status === 400);

const pull = await send(signedRequest(alice, 'sync/v2/pull', null));
expect('cutover advertised on pull', pull, r => r.status === 200 && r.json.signatures_required === true);

const route = crypto.randomUUID();
const aliceEncryptionKey = crypto.randomBytes(32).toString('hex');
const malloryEncryptionKey = crypto.randomBytes(32).toString('hex');
expect('owner Fellowship key registered', await send(signedRequest(alice, 'encryption/register', { public_key: aliceEncryptionKey })), r => r.status === 200);
expect('companion Fellowship key registered', await send(signedRequest(mallory, 'encryption/register', { public_key: malloryEncryptionKey })), r => r.status === 200);
const invitation = await send(signedRequest(alice, 'invite', {
  project_id: route, project_name: '[encrypted]', invitee_identity: mallory.publicHex,
  role: 'Companion', routing_id: route,
  inviter_encryption_key: aliceEncryptionKey,
  key_nonce: crypto.randomBytes(12).toString('base64'),
  key_ciphertext: crypto.randomBytes(48).toString('base64'),
  project_name_nonce: crypto.randomBytes(12).toString('base64'),
  project_name_ciphertext: crypto.randomBytes(48).toString('base64'),
  project_id_nonce: crypto.randomBytes(12).toString('base64'),
  project_id_ciphertext: crypto.randomBytes(48).toString('base64'),
}));
expect('encrypted invitation created', invitation, r => r.status === 200 && Boolean(r.json.invite_id));
const concurrentAccepts = await Promise.all([
  send(signedRequest(mallory, 'accept', { invite_id: invitation.json.invite_id, username: 'Concurrent tester' })),
  send(signedRequest(mallory, 'accept', { invite_id: invitation.json.invite_id, username: 'Concurrent tester' })),
]);
for (const [index, result] of concurrentAccepts.entries()) {
  expect(`simultaneous invitation acceptance ${index + 1}`, result,
    r => r.status === 200 && r.json.status === 'success');
}

// Acceptance may finish before the inviter publishes the first full snapshot.
// The snapshot must still fan project content out to the now-active member.
const compoundMemberId = `${crypto.randomUUID()}__${mallory.publicHex}`;
const postAcceptSnapshotEvent = makeProjectEvent(alice, route, 'project_member', compoundMemberId);
expect('post-accept snapshot published',
  await send(signedRequest(alice, 'sync/v2/snapshot', [postAcceptSnapshotEvent])),
  r => r.status === 200 && r.json.replaced === 1);
const postAcceptPull = await send(signedRequest(mallory, 'sync/v2/pull', null));
expect('post-accept snapshot delivered to companion', postAcceptPull,
  r => r.status === 200 && r.json.events.some(x =>
    x.id === postAcceptSnapshotEvent.id && x.entity_id === compoundMemberId));

expect('Companion ordinary Quest write accepted',
  await send(signedRequest(mallory, 'sync/v2/push', [makeProjectEvent(mallory, route, 'task_status')])),
  r => r.status === 200 && r.json.pushed === 1);
expect('Companion Quest dependency write accepted',
  await send(signedRequest(mallory, 'sync/v2/push', [makeProjectEvent(mallory, route, 'task_dependency')])),
  r => r.status === 200 && r.json.pushed === 1);
expect('Companion assignment administration rejected',
  await send(signedRequest(mallory, 'sync/v2/push', [makeProjectEvent(mallory, route, 'task_assignment')])),
  r => r.status === 403);
expect('Owner assignment administration accepted',
  await send(signedRequest(alice, 'sync/v2/push', [makeProjectEvent(alice, route, 'task_assignment')])),
  r => r.status === 200 && r.json.pushed === 1);

const newRoute = crypto.randomUUID();
expect('encrypted companion removal and route rotation', await send(signedRequest(alice, 'project/remove-member', {
  old_routing_id: route, new_routing_id: newRoute, removed_identity: mallory.publicHex,
  sender_encryption_key: aliceEncryptionKey,
  envelopes: [{ recipient_identity: alice.publicHex,
    key_nonce: crypto.randomBytes(12).toString('base64'),
    key_ciphertext: crypto.randomBytes(48).toString('base64') }],
})), r => r.status === 200 && r.json.routing_id === newRoute);

const revocations = await send(signedRequest(mallory, 'project/revocations', null, 'GET'));
expect('removed client receives stale-route revocation', revocations,
  r => r.status === 200 && r.json.some(x => x.routing_id === route && x.replacement_routing_id === newRoute));

function projectEvent(who, routingId) {
  return makeProjectEvent(who, routingId);
}
expect('removed client old-route write rejected',
  await send(signedRequest(mallory, 'sync/v2/push', [projectEvent(mallory, route)])),
  r => r.status === 403);
const replacementEvent = projectEvent(alice, newRoute);
expect('owner replacement-route write accepted',
  await send(signedRequest(alice, 'sync/v2/push', [replacementEvent])),
  r => r.status === 200 && r.json.pushed === 1);
const removedPull = await send(signedRequest(mallory, 'sync/v2/pull', null));
expect('replacement-route event not delivered to removed client', removedPull,
  r => r.status === 200 && !r.json.events.some(x => x.id === replacementEvent.id));
console.log('All live sync-v2 security tests passed.');
