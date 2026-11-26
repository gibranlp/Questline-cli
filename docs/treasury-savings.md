# Treasury accounts and savings

New-entry account and transfer-destination choices use user-created accounts
(`is_default=false`), including zero-balance accounts. Built-in Spending,
Savings and Cash accounts remain in storage for historical transactions but
are excluded from new-entry choices. Editing an existing transaction preserves
its original legacy account links. Create accounts from Treasury → E → A in
the terminal app. Filtering uses metadata, so a user-created account named
“Savings” remains a valid choice.

A Savings account has three additional settings:

- Availability: Available or Invested.
- Interest rate: a percentage with up to two decimal places.
- Rate period: Monthly or Annual.

Use the terminal account editor or the widget account card's Savings settings
button. Saving settings uses account version checks and the existing encrypted
sync/history path. Existing accounts migrate to Available, 0%, Annual; no real
investment status or rate is guessed. Rates are stored as integer basis points
(12.50% = 1250), with a supported range of 0–10000%.

Estimated interest for one selected period is the positive recorded balance
multiplied by the configured percentage, rounded down to a currency minor unit.
For example, a balance of 1,000 at 1.25% monthly has estimated interest of 12.50
for one month. The estimate assumes an unchanged principal; it does not model
compounding, fees, taxes, deposits or withdrawals. No interest transaction,
background accrual, balance update or XP reward is created by the estimate.
Record actual interest received as an ordinary Income entry when appropriate.

Treasury totals separate:

- **Available:** confirmed income minus paid expenses, approved commitments and
  all balances held in Savings accounts.
- **Saved:** all Savings account balances.
- **Invested:** the subset of Saved marked Invested.
- **Available savings:** the subset of Saved marked Available.

Invested and Available savings are parts of Saved, not additional assets to add
to it. The widget groups totals by currency. Transfers between accounts change
where money is held; they do not count as income or expenses. Moving funds into
Savings reduces Available and increases Saved. Moving them back does the
reverse. Investment-rate settings never change those recorded totals.

Schema version 5 adds the account metadata with safe defaults and retains
account IDs and ledger references. Legacy encrypted payloads lacking the new
metadata preserve existing savings settings when received by an updated client.
Older clients can still read the additive schema, but must be updated to display
or configure savings and to show the new Available calculation.
