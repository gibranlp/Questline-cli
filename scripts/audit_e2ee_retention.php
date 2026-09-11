<?php
// Reports counts only. It never prints stored messages, payloads, or ciphertext.
$loader = $argv[1] ?? '';
if (!$loader || !is_file($loader)) throw new RuntimeException('load_env.php path required');
require $loader;
$options = [PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION];
$legacy = new PDO('mysql:host='.getenv('DB_HOST').';dbname='.getenv('DB_NAME').';charset=utf8mb4',
    getenv('DB_USER'), getenv('DB_PASS'), $options);
$e2ee = new PDO('mysql:host='.getenv('E2EE_DB_HOST').';dbname='.getenv('E2EE_DB_NAME').';charset=utf8mb4',
    getenv('E2EE_DB_USER'), getenv('E2EE_DB_PASSWORD'), $options);

$checks = [
    'migrated_accounts_with_legacy_backup' => "SELECT COUNT(*) FROM backups b JOIN users u ON u.id=b.user_id WHERE COALESCE(u.sync_protocol,1)>=2",
    'migrated_accounts_with_legacy_sync_rows' => "SELECT COUNT(DISTINCT s.user_id) FROM sync_events s JOIN users u ON u.id=s.user_id WHERE COALESCE(u.sync_protocol,1)>=2",
    'api_log_private_field_markers' => "SELECT COUNT(*) FROM api_logs WHERE message REGEXP 'markdown_content|journal_content|backup_data|\"(title|description|content)\"[[:space:]]*:'",
];
foreach ($checks as $name => $sql) echo $name.'='.$legacy->query($sql)->fetchColumn().PHP_EOL;
echo 'encrypted_events_json_shaped_ciphertext='.
    $e2ee->query("SELECT COUNT(*) FROM sync_v2_events WHERE LEFT(TRIM(ciphertext),1) IN ('{','[')")->fetchColumn().PHP_EOL;
echo 'encrypted_events_total='.$e2ee->query('SELECT COUNT(*) FROM sync_v2_events')->fetchColumn().PHP_EOL;

$pattern = '/markdown_content|journal_content|backup_data|"(?:title|description|content)"\s*:/i';
foreach (array_slice($argv, 2) as $file) {
    if (!is_readable($file)) continue;
    $matches = 0;
    $handle = fopen($file, 'rb');
    while (($line = fgets($handle)) !== false) if (preg_match($pattern, $line)) $matches++;
    fclose($handle);
    echo 'file_private_field_markers['.basename($file).']='.$matches.PHP_EOL;
}
