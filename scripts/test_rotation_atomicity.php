<?php
// Live database integration test for interrupted encrypted Fellowship rotation.
// Uses isolated random routes/accounts and removes its fixtures before exiting.
$loader = $argv[1] ?? '';
if (!$loader || !is_file($loader)) throw new RuntimeException('load_env.php path required');
require $loader;
$pdo = new PDO(
    'mysql:host=' . getenv('E2EE_DB_HOST') . ';dbname=' . getenv('E2EE_DB_NAME') . ';charset=utf8mb4',
    getenv('E2EE_DB_USER'), getenv('E2EE_DB_PASSWORD'),
    [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION]
);

function test_uuid(): string {
    $h = bin2hex(random_bytes(16));
    return substr($h, 0, 8).'-'.substr($h, 8, 4).'-4'.substr($h, 13, 3).'-a'.substr($h, 17, 3).'-'.substr($h, 20, 12);
}

$oldRoute = test_uuid();
$newRoute = test_uuid();
$owner = test_uuid();
$removed = test_uuid();

try {
    $member = $pdo->prepare('INSERT INTO project_v2_members (routing_id, account_id, role) VALUES (?, ?, ?)');
    $member->execute([$oldRoute, $owner, 'Owner']);
    $member->execute([$oldRoute, $removed, 'Companion']);

    // This fixture forces the envelope insert to fail only after replacement
    // membership has been inserted inside the transaction.
    $envelope = $pdo->prepare('INSERT INTO project_v2_key_envelopes
        (id, old_routing_id, new_routing_id, recipient_account_id, sender_encryption_key, key_nonce, key_ciphertext)
        VALUES (?, ?, ?, ?, ?, ?, ?)');
    $envelope->execute([test_uuid(), $oldRoute, $newRoute, $owner, str_repeat('a', 64),
        base64_encode(random_bytes(12)), base64_encode(random_bytes(48))]);

    $failed = false;
    $pdo->beginTransaction();
    try {
        $member->execute([$newRoute, $owner, 'Owner']);
        $envelope->execute([test_uuid(), $oldRoute, $newRoute, $owner, str_repeat('a', 64),
            base64_encode(random_bytes(12)), base64_encode(random_bytes(48))]);
        $pdo->prepare('INSERT INTO project_v2_retired_routes (routing_id, replacement_routing_id) VALUES (?, ?)')
            ->execute([$oldRoute, $newRoute]);
        $pdo->commit();
    } catch (Throwable $error) {
        $failed = true;
        if ($pdo->inTransaction()) $pdo->rollBack();
    }

    $count = fn(string $sql, array $args) => (function() use ($pdo, $sql, $args) {
        $q = $pdo->prepare($sql); $q->execute($args); return (int)$q->fetchColumn();
    })();
    $oldMembers = $count('SELECT COUNT(*) FROM project_v2_members WHERE routing_id = ?', [$oldRoute]);
    $newMembers = $count('SELECT COUNT(*) FROM project_v2_members WHERE routing_id = ?', [$newRoute]);
    $retired = $count('SELECT COUNT(*) FROM project_v2_retired_routes WHERE routing_id = ?', [$oldRoute]);
    if (!$failed || $oldMembers !== 2 || $newMembers !== 0 || $retired !== 0) {
        throw new RuntimeException("atomicity failure: failed=$failed old=$oldMembers new=$newMembers retired=$retired");
    }
    echo "PASS interrupted rotation rolled back; old route remains active and replacement state is absent\n";
} finally {
    foreach ([
        ['DELETE FROM project_v2_key_envelopes WHERE old_routing_id = ? OR new_routing_id = ?', [$oldRoute, $newRoute]],
        ['DELETE FROM project_v2_retired_routes WHERE routing_id = ? OR replacement_routing_id = ?', [$oldRoute, $newRoute]],
        ['DELETE FROM project_v2_members WHERE routing_id = ? OR routing_id = ?', [$oldRoute, $newRoute]],
    ] as [$sql, $args]) { $q = $pdo->prepare($sql); $q->execute($args); }
}
