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

For an existing sync-v2 database, apply these migrations in order before
deploying signed clients:

1. Apply `006_signed_sync_events.sql` to add durable author signatures and the
   per-account signature cutover state.
2. Apply `007_widen_signed_entity_ids.sql` to widen compound Fellowship entity
   IDs. This migration removes only truncated compound rows that can no longer
   pass signature or AES-GCM verification; an authoritative trusted device must
   perform one full sync afterward to republish them.

Migration `001_sync_v2.sql` already includes both schema changes for a brand-new
database, so do not reapply `006` or `007` after a fresh bootstrap.

During the compatibility window, accounts without a cutover row may still
contain unsigned history. A trusted client cuts an account over by replacing
its encrypted snapshot: every replacement event must carry a valid Ed25519
signature, and the server atomically sets
`account_v2_security.signatures_required = 1`. After that point the server
rejects unsigned writes and clients reject unsigned pulls.

Do not set the cutover flag manually. Replacing the snapshot retires the old
unsigned rows without invalidating signed Fellowship history copied between
member accounts; copied project events retain the original author's public key
and signature.


## Sync reliability and profile transfer upgrade

For an existing encrypted database, apply `008_profile_backups.sql`, then
`009_sync_generations.sql` **before** deploying the updated PHP API. A fresh
`001_sync_v2.sql` bootstrap already includes both tables.

The new profile backup routes store an AES-GCM envelope independently of
`sync_v2_events`. They never replace or prune the live account event history. The backup generation
is authenticated as AES-GCM associated data. Uploads from an old generation are
rejected, and recovery ignores an older backup after an explicit history reset.
The CLI uses these snapshots for transfer-code recovery and then pulls the event
tail through a temporary recovery device ID.

Updated clients check `sync/v2/head` and retain its account history generation.
Explicit history replacement requires `expected_head`; a changed head returns
409 without deleting history. Successful replacement increments the generation. Account replacement preserves
project-scoped history and its original authors: an Observer cannot republish
that history, so deleting it would make subsequent recovery incomplete.
After a replacement, pushes must provide the matching `expected_generation`.
Missing preconditions return 428, and a stale generation returns 409. Update
clients before using reset: older clients must not replace history as a side
effect of exporting a profile. An affected updated client can restore into a
staging database; unsent local changes should be exported before replacing its
profile.

Deploy the PHP API and browser build together, then release the CLI. Do not run
live reset/security tests against production accounts. The security script now
defaults to localhost and creates disposable identities. Validation used a local
MariaDB 11.4 container and PHP 8.3 with `pdo_mysql`, with separate disposable
legacy and encrypted databases, and only `api/index.php` and `load_env.php`
mounted (no environment/credential files).

Against that disposable API:

```sh
QUESTLINE_API_URL=http://127.0.0.1:18092/api/ node scripts/test_sync_v2_security.mjs
QUESTLINE_TEST_API_URL=http://127.0.0.1:18092/api/ cargo test --lib encrypted_cloud_transfer_preserves_source -- --ignored
```

Normal local checks:

```sh
questline_test_state=$(mktemp -d /tmp/questline-sync-tests.XXXXXX)
QUESTLINE_STORAGE_DIR="$questline_test_state" cargo test --lib --bins -- --test-threads=2
npm --prefix webapp test
npm --prefix webapp run build
php -l server/api/index.php
php -l webapp/api/index.php
```
