// Encrypted Fellowship (shared projects) for the browser.
//
// All Fellowship routing lives in the encrypted questlinecli.com database
// (gibranlp_QuestlineE); the webapp API does not carry project scope. So every
// call here targets QUESTLINE_API_BASE directly. Project content flows as
// encrypted project-scoped sync events (see sync.js); this module handles the
// key material: X25519 registration, invitations, and rotation envelopes.

import { get } from 'svelte/store';
import { ApiClient, QUESTLINE_API_BASE } from './api.js';
import {
  fellowshipPublicKeyHex, generateProjectKey, wrapProjectKey, unwrapProjectKey,
  encryptProjectPayload, decryptProjectPayload, projectKeyToHex, projectKeyFromHex,
} from './crypto.js';
import {
  saveProjectKey, deleteProjectKey, getProjectKeyByRouting, getActiveProjectKeyByProject, markRouteRetired,
} from './db.js';
import { projects, addToast } from './store.js';
import { saveEntity } from './db.js';

function qClient(identity) {
  return new ApiClient(identity, QUESTLINE_API_BASE);
}

// ── Registration ──────────────────────────────────────────────────────────────
// Publish this account's X25519 public key so others can invite us. Idempotent;
// we register once per identity per browser (a cheap UPDATE server-side).

export async function ensureEncryptionKeyRegistered(identity) {
  const flag = `questline_fellowship_registered_${identity.public_key}`;
  if (localStorage.getItem(flag) === '1') return;
  try {
    const publicKey = await fellowshipPublicKeyHex(identity);
    await qClient(identity).post('encryption/register', { public_key: publicKey });
    localStorage.setItem(flag, '1');
  } catch (err) {
    console.warn('[fellowship] key registration failed:', err);
  }
}

async function fetchCompanionKey(identity, companionIdentityHex) {
  const res = await qClient(identity).get('encryption/key', { identity: companionIdentityHex });
  const key = res?.public_key;
  if (!/^[0-9a-f]{64}$/i.test(key || '')) {
    throw new Error('Companion has not enabled encrypted Fellowship yet');
  }
  return key.toLowerCase();
}

// ── Sending an invitation ───────────────────────────────────────────────────
// Ensures the project has a key + routing_id, wraps the key to the invitee, and
// encrypts the project name/id. Marks the project shared locally so subsequent
// edits sync as project-scoped events.

export async function sendInvitation(identity, project, inviteeIdentityHex, role = 'Companion') {
  if (!/^[0-9a-f]{64}$/i.test(inviteeIdentityHex || '')) {
    throw new Error('Enter a valid companion identity (64 hex characters)');
  }
  const inviteeKey = await fetchCompanionKey(identity, inviteeIdentityHex);

  // Reuse the project's existing key/route, or mint one on first share.
  let rec = await getActiveProjectKeyByProject(project.id);
  let projectKey, routingId;
  if (rec) {
    projectKey = projectKeyFromHex(rec.key_hex);
    routingId = rec.routing_id;
  } else {
    projectKey = generateProjectKey();
    routingId = crypto.randomUUID();
    await saveProjectKey({ routing_id: routingId, project_id: project.id, key_hex: projectKeyToHex(projectKey) });
    // Register ourselves as Owner on the route (the invite call does this server-side).
  }

  const keyEnv = await wrapProjectKey(identity, inviteeKey, routingId, projectKey);
  const nameEnc = await encryptProjectPayload(projectKey, project.name || '', `questline/fellowship/name/v1/${routingId}`);
  const idEnc   = await encryptProjectPayload(projectKey, String(project.id), `questline/fellowship/id/v1/${routingId}`);
  const inviterKey = await fellowshipPublicKeyHex(identity);

  await qClient(identity).post('invite', {
    project_id: project.id,
    project_name: '[encrypted]',
    invitee_identity: inviteeIdentityHex.toLowerCase(),
    role,
    routing_id: routingId,
    inviter_encryption_key: inviterKey,
    key_nonce: keyEnv.nonce,
    key_ciphertext: keyEnv.ciphertext,
    project_name_nonce: nameEnc.nonce,
    project_name_ciphertext: nameEnc.ciphertext,
    project_id_nonce: idEnc.nonce,
    project_id_ciphertext: idEnc.ciphertext,
  });

  // Flip the project to shared locally so future edits become project-scoped.
  await markProjectShared(project.id, true);
  return { routing_id: routingId };
}

// ── Pending invitations ───────────────────────────────────────────────────────
// List invitations addressed to us and decrypt their name/id locally without
// revealing anything to the server. A corrupt/forged envelope is dropped.

export async function listPendingInvitations(identity) {
  let rows;
  try {
    rows = await qClient(identity).get('pending');
  } catch (err) {
    console.warn('[fellowship] pending list failed:', err);
    return [];
  }
  const out = [];
  for (const inv of (rows || [])) {
    if (!inv.routing_id || !inv.inviter_encryption_key) continue; // legacy plaintext invite — skip
    try {
      const keyBytes = await unwrapProjectKey(
        identity, inv.inviter_encryption_key, inv.routing_id, inv.key_nonce, inv.key_ciphertext,
      );
      const name = await decryptProjectPayload(
        keyBytes, inv.project_name_nonce, inv.project_name_ciphertext,
        `questline/fellowship/name/v1/${inv.routing_id}`,
      );
      const projectId = await decryptProjectPayload(
        keyBytes, inv.project_id_nonce, inv.project_id_ciphertext,
        `questline/fellowship/id/v1/${inv.routing_id}`,
      );
      out.push({
        id: inv.id,
        routing_id: inv.routing_id,
        role: inv.role,
        inviter_username: inv.inviter_username,
        inviter_identity: inv.inviter_identity,
        project_name: name,
        project_id: projectId,
        _key_hex: projectKeyToHex(keyBytes),
      });
    } catch (err) {
      console.warn('[fellowship] dropping unverifiable invitation', inv.id, err);
    }
  }
  return out;
}

// ── Accepting an invitation ────────────────────────────────────────────────────
// `invite` is an item returned by listPendingInvitations (already validated). We
// persist the project key, then POST accept. Idempotent server-side.

export async function acceptInvitation(identity, invite, username) {
  await saveProjectKey({
    routing_id: invite.routing_id,
    project_id: invite.project_id,
    key_hex: invite._key_hex,
  });
  try {
    await qClient(identity).post('accept', { invite_id: invite.id, username: username || '' });
  } catch (error) {
    // Do not retain attacker-supplied or stale invitation material when the
    // authoritative server refuses the invitation. A successful response is
    // idempotent, so crash recovery after acceptance remains safe.
    await deleteProjectKey(invite.routing_id).catch(() => {});
    throw error;
  }
  // The project row itself arrives as a project-scoped sync event on the next pull.
  return invite.project_id;
}

// ── Rotation delivery: pull new keys + revocations before a project pull ───────

export async function processKeyEnvelopesAndRevocations(identity) {
  const client = qClient(identity);

  try {
    const envelopes = await client.get('project/key-envelopes');
    for (const env of (envelopes || [])) {
      if (await getProjectKeyByRouting(env.new_routing_id)) continue; // already migrated
      const oldRec = await getProjectKeyByRouting(env.old_routing_id);
      if (!oldRec) continue; // we don't know this project — ignore
      try {
        const keyBytes = await unwrapProjectKey(
          identity, env.sender_encryption_key, env.new_routing_id, env.key_nonce, env.key_ciphertext,
        );
        await saveProjectKey({
          routing_id: env.new_routing_id,
          project_id: oldRec.project_id,
          key_hex: projectKeyToHex(keyBytes),
        });
        await markRouteRetired(env.old_routing_id);
      } catch (err) {
        console.warn('[fellowship] could not unwrap rotation envelope', env.new_routing_id, err);
      }
    }
  } catch (err) {
    console.warn('[fellowship] key-envelopes failed:', err);
  }

  try {
    const revocations = await client.get('project/revocations');
    for (const rev of (revocations || [])) {
      const rec = await getProjectKeyByRouting(rev.routing_id);
      if (!rec || rec.retired) continue;
      await markRouteRetired(rev.routing_id);
      // If we never received a replacement key, we were removed: demote to a
      // private local copy (future edits sync under the account key).
      const replacement = await getProjectKeyByRouting(rev.replacement_routing_id);
      if (!replacement) {
        await markProjectShared(rec.project_id, false);
        addToast('A shared campaign is now a private local copy.', 'warning');
      }
    }
  } catch (err) {
    console.warn('[fellowship] revocations failed:', err);
  }
}

// ── Owner: remove a member (rotate the project key + route) ────────────────────

export async function removeMember(identity, projectId, removedIdentityHex) {
  const rec = await getActiveProjectKeyByProject(projectId);
  if (!rec) throw new Error('This campaign is not shared');
  const oldRoute = rec.routing_id;

  const members = await qClient(identity).get('project/rotation-members', { routing_id: oldRoute });
  const roster = Array.isArray(members) ? members : [];
  const target = roster.find(m => (m.identity || '').toLowerCase() === removedIdentityHex.toLowerCase());
  if (!target) throw new Error('That companion is not a member of this campaign');

  const newKey = generateProjectKey();
  const newRoute = crypto.randomUUID();
  const senderKey = await fellowshipPublicKeyHex(identity);

  const envelopes = [];
  for (const member of roster) {
    if ((member.identity || '').toLowerCase() === removedIdentityHex.toLowerCase()) continue;
    if (!/^[0-9a-f]{64}$/i.test(member.encryption_public_key || '')) {
      throw new Error('A remaining companion has no encryption key; rotation aborted');
    }
    const env = await wrapProjectKey(identity, member.encryption_public_key, newRoute, newKey);
    envelopes.push({
      recipient_identity: member.identity,
      key_nonce: env.nonce,
      key_ciphertext: env.ciphertext,
    });
  }

  await qClient(identity).post('project/remove-member', {
    old_routing_id: oldRoute,
    new_routing_id: newRoute,
    removed_identity: removedIdentityHex.toLowerCase(),
    sender_encryption_key: senderKey,
    envelopes,
  });

  await saveProjectKey({ routing_id: newRoute, project_id: projectId, key_hex: projectKeyToHex(newKey) });
  await markRouteRetired(oldRoute);
  return { routing_id: newRoute };
}

// List current companions on a project's active route (for the roster / removal UI).
export async function listRotationMembers(identity, projectId) {
  const rec = await getActiveProjectKeyByProject(projectId);
  if (!rec) return [];
  try {
    const members = await qClient(identity).get('project/rotation-members', { routing_id: rec.routing_id });
    return Array.isArray(members) ? members : [];
  } catch (err) {
    console.warn('[fellowship] rotation-members failed:', err);
    return [];
  }
}

// ── Local project state helper ────────────────────────────────────────────────

async function markProjectShared(projectId, shared) {
  const map = get(projects);
  const proj = map.get(projectId);
  if (!proj || proj.is_shared === shared) return;
  const updated = { ...proj, is_shared: shared };
  projects.update(m => { const n = new Map(m); n.set(projectId, updated); return n; });
  try { await saveEntity('projects', updated); } catch {}
}
