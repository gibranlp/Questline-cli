# Encrypted sync-v2 deployment

This migration creates a ciphertext-only event store in the separate
`gibranlp_QuestlineE` database. It does not touch the legacy Questline database.

1. Create the `gibranlp_QuestlineE` database and grant the
   `gibranlp_QuestlineE` user access to it.
2. Apply `001_sync_v2.sql` to that database.
3. Set the `E2EE_DB_*` variables shown in `server/.env.example` in every API
   deployment that serves `sync/v2/*`.
4. Deploy the server before encrypted clients. Confirm `sync/v2/pull` returns an
   empty envelope for a test account.
5. On the one device chosen as authoritative, use Questline's Cloud Backup
   action. It queues and pushes a full local-state snapshot, now encrypted by
   the client. Do not run this step concurrently on multiple old devices.
6. Restore on a second trusted device and compare the local entity counts and
   representative content before enabling the other devices.

Do not copy legacy `sync_events.payload` rows into this database. Only a trusted
client holding the Transfer Code/private identity can create valid ciphertext.
Keep the legacy database unchanged until rollback is no longer required.

## Encrypted Fellowship upgrade

For an existing sync-v2 installation, apply these before deploying the
Fellowship-capable server/client:

1. Apply `004_legacy_fellowship_key_envelopes.sql` to the legacy Questline
   database. It stores only X25519 public keys, opaque routing identifiers, and
   encrypted invitation envelopes.
2. Apply `005_encrypted_fellowship_routing.sql` to `gibranlp_QuestlineE`. This
   adds opaque routing columns and the route membership table.
3. Deploy `server/api/index.php`, then deploy the CLI.

Do not apply `005_encrypted_fellowship_routing.sql` to a brand-new database
created from the current `001_sync_v2.sql`; the current bootstrap already
contains those columns. The migrations deliberately avoid `ADD COLUMN IF NOT
EXISTS` for compatibility with the MySQL version used by the current host.

The web app creates its own account-only `sync_v2_events` table on first request.
Encrypted Fellowship is now supported in the browser: it stores project keys and
X25519 envelopes and routes project-scoped events through `questlinecli.com`
(the webapp DB stays account-only). The browser never falls back to plaintext for
a shared project.

Encrypted member removal is owner-only and fail-closed. It creates a new project
key and opaque route, requires a replacement key envelope for every remaining
member (including the owner for crash recovery), retires the old route, and only
then removes the local member. A removed member keeps content downloaded before
removal but cannot receive or publish events on the replacement route.

Removed clients discover retired routes through `project/revocations` before a
sync pull. Their already-downloaded campaign is preserved as a private copy, old
Fellowship outbox rows are cleared, and future edits use account encryption so a
retired route cannot block unrelated private synchronization. This endpoint is a
server-code deployment only and requires no migration beyond the current `005`.

## Durable event-signature cutover

Apply `006_signed_sync_events.sql` before deploying signed clients. During the
compatibility window, accounts without a cutover row may still contain unsigned
history. A trusted client cuts an account over by replacing its encrypted
snapshot: every replacement event must carry a valid Ed25519 signature, and the
server atomically sets `account_v2_security.signatures_required = 1`. After that
point the server rejects unsigned writes and clients reject unsigned pulls.

Do not set the cutover flag manually. Replacing the snapshot retires the old
unsigned rows without invalidating signed Fellowship history copied between
member accounts; copied project events retain the original author's public key
and signature.
