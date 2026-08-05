# Encrypted snapshot replacement

No SQL schema change is required after `001_sync_v2.sql`.

Deploy the updated `server/api/index.php` to enable `sync/v2/snapshot`. The
endpoint validates the complete encrypted batch before opening a transaction,
then deletes and replaces only the authenticated account's `sync_v2_events`
rows. A failed insert rolls the transaction back, preserving the prior snapshot.

After the encrypted snapshot commits, the legacy account is marked protocol 2
and that account's plaintext `backups` row is deleted. Other accounts and all
legacy sync history remain untouched.

Do not manually truncate `sync_v2_events`; other accounts may share the table.
