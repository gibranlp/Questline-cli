# Treasury review — 2026-10-03

## Result

The CLI supports multiple accounts of each existing type: Spending, Savings, Cash, and Other. Accounts are independent of transaction categories: any account can hold entries in any compatible Income, Expense, Transfer, or Adjustment category. No schema migration is needed for the requested account management.

`A` adds an account. `E` opens a scrolling account manager showing every account, type, and balance; Enter edits the selected account. Default accounts can also be renamed or reclassified. In the transaction editor (`e`), changing Account reassigns an existing transaction; changing Destination reassigns a transfer's receiving side. Shared campaigns retain Owner/Steward restrictions for editing paid entries and managing accounts.

## Changes made

- Added account editing with optimistic version checks, immutable account/campaign identity, preserved creation timestamps and default markers, and case-insensitive duplicate-name validation.
- Account creation/editing commits the account, audit snapshot, and sync change together in one SQLite transaction.
- Added the account manager so custom accounts beyond the first three are visible and reachable. The compact dashboard continues showing three balances and points to `E` for the full list.
- Account form errors keep the form open and display the reason.
- Fixed a transfer validation hole: a legacy null source means the default Spending account, so that account cannot also be the destination.
- Removed initialization's silent, unversioned UPDATE of legacy null account links. Existing null links consistently resolve to the deterministic default Spending account in balance calculations and account filtering, without making an unlogged local mutation. An explicit reassignment goes through the normal versioned, audited, synced entry update.
- JSON export now includes every ledger entry, including paid transactions and their account references. Previously it included totals, account balances, and outstanding payments but omitted most transactions.
- Updated help and README instructions.

## Verification

`cargo test --lib`: 396 passed, zero failures.

`cargo check --bins`: passed.

Treasury-specific coverage includes account edits, duplicate/blank names, stale edits, default-account persistence, paid-entry reassignment, preserved payment date and authorship, unchanged campaign totals, recalculated balances, transfer neutrality, budget thresholds, currency relabeling, monthly metrics, migrations, permissions, and JSON export.

Sync tests use the real SyncEngine and encryption with an in-memory event-log provider. Both private account-key sync and shared Fellowship project-key sync deliver custom accounts, account edits, and paid-entry moves to another database. Repeated sync preserves versions and does not duplicate transactions. Existing tests verify dependency retries, encrypted envelopes, signatures, and Observer write suppression. These checks do not contact the deployed server or validate its installed version.

## Remaining review findings

1. **Shared Treasury governance is partly client-enforced.** In `server/api/index.php`, the encrypted push handler's administrative-entity list includes task assignments and project membership, but not Treasury accounts, categories, or budgets. A modified Companion client could submit these encrypted entities despite the official CLI's Owner/Steward checks. Ledger ownership and status-transition restrictions also rely on client checks because the server cannot inspect ciphertext. This change preserves the existing permission model; server hardening needs corresponding outgoing-sync handling so initialization writes cannot cause rejected batches.

2. **Concurrent same-name account creation can remain deferred.** The SQLite schema requires unique account names within a campaign, while custom accounts receive random UUIDs. Two offline devices independently creating the same name can produce different IDs; the incoming INSERT then conflicts and is retried. Ordinary creation, edits, and reassignment pass sync tests, but this collision still needs an explicit merge/conflict workflow.

3. **The web app has no Treasury data/UI parity.** Its current IndexedDB entity list and sync application logic do not implement Treasury accounts/ledger entities. The requested workflow is implemented in the Rust CLI; CLI-to-web Treasury editing is not provided by this change.

4. **Adjustment entries do not affect monetary totals or account balances.** The balance calculation handles confirmed Income, paid Expense, and paid Transfer. Adjustment categories exist but no signed correction model defines their financial effect. A reconciliation workflow needs a separate amount/direction decision; changing that meaning silently would alter existing balances.

5. **Whole-row sync resolves concurrent edits by updated_at.** Local account and entry editors reject stale versions, but simultaneous offline edits on separate devices still use the existing timestamp-based incoming-entity rule. There is no field-level merge or interactive conflict selection.

6. **The local shell API has narrower Treasury controls.** Its create/update entry actions cover a subset of the CLI editor and do not expose account management or account reassignment. The terminal workflow and encrypted CLI sync were the implementation scope here.

There was no deployment, installation, or live-account mutation. Existing unrelated workspace edits were preserved.
