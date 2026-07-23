<?php
// ─────────────────────────────────────────────────────────────────────────────
// webapp-api/index.php — Backend PHP propio del webapp, con su propia DB
// Sirve datos desde gibranlp_webappquest en lugar de questlinecli.com
// ─────────────────────────────────────────────────────────────────────────────

header("Content-Type: application/json; charset=UTF-8");
header("Access-Control-Allow-Origin: *");
header("Access-Control-Allow-Headers: Content-Type, X-Identity, X-User-Id, X-Device-Id, X-Timestamp, X-Nonce, X-Signature");
header("Access-Control-Allow-Methods: GET, POST, OPTIONS");
header("X-Content-Type-Options: nosniff");
header("X-Frame-Options: DENY");
header("Referrer-Policy: no-referrer");

if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') { exit(0); }

$host = $_SERVER['HTTP_HOST'] ?? '';
$isLocal = str_contains($host, 'localhost') || str_contains($host, '127.0.0.1');
if (!$isLocal && (empty($_SERVER['HTTPS']) || $_SERVER['HTTPS'] === 'off')) {
    http_response_code(403);
    echo json_encode(["error" => "HTTPS required"]);
    exit;
}

// ── Parse route early for public endpoints ────────────────────────────────────
$requestUri  = $_SERVER['REQUEST_URI'] ?? '';
$apiPath     = parse_url($requestUri, PHP_URL_PATH);
$pathSegs    = explode('/api/', $apiPath);
$route       = isset($pathSegs[1]) ? rtrim($pathSegs[1], '/') : '';
if (str_starts_with($route, 'index.php/')) { $route = substr($route, 10); }
elseif ($route === 'index.php') { $route = ''; }
if (empty($route)) { $route = $_GET['route'] ?? ''; }

// ── Health check — public, no auth, no DB needed ─────────────────────────────
if ($route === 'health') {
    echo json_encode(["status" => "ok", "service" => "questline-webapp-api"]);
    exit;
}

// ── Load .env from the same directory (self-contained — no load_env.php needed) ─
// (loaded before debug-status so the env report is accurate)
$_envFile = __DIR__ . '/.env';
if (file_exists($_envFile)) {
    foreach (file($_envFile, FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES) ?: [] as $_envLine) {
        $_envLine = trim($_envLine);
        if (!$_envLine || $_envLine[0] === '#' || !str_contains($_envLine, '=')) continue;
        [$_envKey, $_envVal] = explode('=', $_envLine, 2);
        $_envKey = trim($_envKey);
        $_envVal = trim($_envVal);
        $_len = strlen($_envVal);
        if ($_len >= 2 && (
            ($_envVal[0] === '"'  && $_envVal[$_len - 1] === '"')  ||
            ($_envVal[0] === "'" && $_envVal[$_len - 1] === "'")
        )) { $_envVal = substr($_envVal, 1, -1); }
        putenv("$_envKey=$_envVal");
        $_ENV[$_envKey] = $_envVal;
    }
}
unset($_envFile, $_envLine, $_envKey, $_envVal, $_len);

// ── DB connection ─────────────────────────────────────────────────────────────
$db_host = getenv('WEBAPP_DB_HOST') ?: 'localhost';
$db_name = getenv('WEBAPP_DB_NAME');
$db_user = getenv('WEBAPP_DB_USER');
$db_pass = getenv('WEBAPP_DB_PASS');

// ── Debug status — reports env + DB health without leaking credentials ────────
if ($route === 'debug-status') {
    $report = [
        'env_file'     => file_exists(__DIR__ . '/.env') ? 'found' : 'MISSING',
        'db_name_set'  => !empty($db_name),
        'db_user_set'  => !empty($db_user),
        'db_pass_set'  => !empty($db_pass),
        'sodium'       => extension_loaded('sodium'),
    ];
    try {
        $dbh = new PDO("mysql:host=$db_host;dbname=$db_name;charset=utf8mb4", $db_user, $db_pass);
        $report['db'] = 'connected';
        $report['tables'] = $dbh->query("SHOW TABLES")->fetchAll(PDO::FETCH_COLUMN);
    } catch (PDOException $e) {
        $report['db'] = 'FAILED';
        $report['db_error'] = $e->getMessage();
    }
    echo json_encode($report);
    exit;
}

if (!$db_name || !$db_user || !$db_pass) {
    error_log("[WebApp API] Missing DB credentials — check .env");
    http_response_code(500);
    echo json_encode(["error" => "Server misconfiguration"]);
    exit;
}

try {
    $pdo = new PDO("mysql:host=$db_host;dbname=$db_name;charset=utf8mb4", $db_user, $db_pass, [
        PDO::ATTR_ERRMODE            => PDO::ERRMODE_EXCEPTION,
        PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC,
    ]);
    setup_tables($pdo);
} catch (PDOException $e) {
    error_log("[WebApp API] DB connect failed: " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Service temporarily unavailable"]);
    exit;
}

// ── Proxy: webapp login/register — these live on questlinecli.com ─────────────
$questlineApi = 'https://questlinecli.com/api/';

if ($route === 'webapp/login') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); echo json_encode(["error" => "Method not allowed"]); exit; }
    $raw  = file_get_contents('php://input');
    $resp = curl_post_json($questlineApi . '?route=webapp/login', $raw);
    http_response_code($resp['code']);
    echo $resp['body'];
    exit;
}

if ($route === 'webapp/register') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); echo json_encode(["error" => "Method not allowed"]); exit; }
    $raw  = file_get_contents('php://input');
    $resp = curl_post_json($questlineApi . '?route=webapp/register', $raw);
    http_response_code($resp['code']);
    echo $resp['body'];
    exit;
}

if ($route === 'webapp/check-email') {
    $email = urlencode($_GET['email'] ?? '');
    $resp  = curl_get_json($questlineApi . "?route=webapp/check-email&email=$email");
    http_response_code($resp['code']);
    echo $resp['body'];
    exit;
}

// ── Webhook ingest — receives sync events from questlinecli.com ───────────────
// Verified with HMAC-SHA256; no Ed25519 needed because this is server-to-server
if ($route === 'webhook/ingest') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); echo json_encode(["error" => "Method not allowed"]); exit; }

    $raw  = file_get_contents('php://input');
    $data = json_decode($raw, true);

    if (!$data || empty($data['user_id']) || empty($data['entity_type']) || empty($data['entity_id'])) {
        http_response_code(400);
        echo json_encode(["error" => "Invalid webhook payload"]);
        exit;
    }

    $ingestUserId = $data['user_id'];

    // Verify HMAC signature if the user has a stored webhook secret
    $secretStmt = $pdo->prepare("SELECT secret FROM webhook_secrets WHERE user_id = ?");
    $secretStmt->execute([$ingestUserId]);
    $secretRow  = $secretStmt->fetch();

    if ($secretRow) {
        $sigHeader = $_SERVER['HTTP_X_QUESTLINE_SIGNATURE'] ?? '';
        $expected  = hash_hmac('sha256', $raw, $secretRow['secret']);
        if (!hash_equals($expected, $sigHeader)) {
            http_response_code(401);
            echo json_encode(["error" => "Invalid webhook signature"]);
            exit;
        }
    }

    // Upsert user so foreign keys work
    $pdo->prepare("INSERT INTO users (id, public_key) VALUES (?, '') ON DUPLICATE KEY UPDATE id=id")
        ->execute([$ingestUserId]);

    // Store the sync event (INSERT IGNORE prevents duplicates)
    $stmt = $pdo->prepare(
        "INSERT IGNORE INTO sync_events (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, '')"
    );
    $stmt->execute([
        $data['event_id']    ?? bin2hex(random_bytes(16)),
        $ingestUserId,
        $data['entity_type'],
        $data['entity_id'],
        $data['operation']   ?? 'upsert',
        $data['content']     ?? '',
        $data['timestamp']   ?? date(DATE_RFC3339),
    ]);

    echo json_encode(["status" => "ok"]);
    exit;
}

// ── Ed25519 auth block — everything below requires valid signature ─────────────
$headers   = getallheaders();
$userId    = $headers['X-User-Id']    ?? $headers['x-user-id']    ?? null;
$identity  = $headers['X-Identity']   ?? $headers['x-identity']   ?? null;
$deviceId  = $headers['X-Device-Id']  ?? $headers['x-device-id']  ?? null;
$timestamp = $headers['X-Timestamp']  ?? $headers['x-timestamp']  ?? null;
$nonce     = $headers['X-Nonce']      ?? $headers['x-nonce']      ?? null;
$signature = $headers['X-Signature']  ?? $headers['x-signature']  ?? null;

if (!$userId || !$identity || !$deviceId || !$timestamp || !$nonce || !$signature) {
    http_response_code(400);
    echo json_encode(["error" => "Missing authentication headers"]);
    exit;
}

if (!preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i', $userId)  ||
    !preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i', $deviceId) ||
    strlen($identity)  !== 64 || !ctype_xdigit($identity)  ||
    strlen($signature) !== 128 || !ctype_xdigit($signature)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid request format"]);
    exit;
}

$requestTime = strtotime($timestamp);
if (!$requestTime || abs(time() - $requestTime) > 300) {
    http_response_code(401);
    echo json_encode(["error" => "Timestamp out of range"]);
    exit;
}

// Nonce replay check
$nonceStripped = str_replace('-', '', $nonce);
if (strlen($nonceStripped) < 16 || strlen($nonceStripped) > 128 || !ctype_xdigit($nonceStripped)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid nonce"]);
    exit;
}

try {
    $stmt = $pdo->prepare("SELECT 1 FROM nonces WHERE user_id = ? AND nonce = ?");
    $stmt->execute([$userId, $nonce]);
    if ($stmt->fetch()) {
        http_response_code(401);
        echo json_encode(["error" => "Replay attack detected"]);
        exit;
    }
    $pdo->prepare("INSERT INTO nonces (user_id, nonce) VALUES (?, ?)")->execute([$userId, $nonce]);
    if (mt_rand(0, 99) === 0) {
        $pdo->exec("DELETE FROM nonces WHERE created_at < DATE_SUB(NOW(), INTERVAL 10 MINUTE)");
    }
} catch (PDOException $e) {
    error_log("[WebApp API] Auth DB error (nonce): " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Auth service error: " . $e->getMessage()]);
    exit;
}

// Signature verification
$body          = file_get_contents('php://input');
$messageToSign = $timestamp . '.' . $nonce . '.' . $body;
if (!verify_ed25519($identity, $signature, $messageToSign)) {
    http_response_code(401);
    echo json_encode(["error" => "Signature verification failed"]);
    exit;
}

// Decode base64-encoded POST body (same pattern as questlinecli.com)
if ($_SERVER['REQUEST_METHOD'] === 'POST' && !empty($body)) {
    $first = $body[0];
    if ($first !== '{' && $first !== '[') {
        $decoded = base64_decode($body, true);
        if ($decoded !== false && ($decoded[0] === '{' || $decoded[0] === '[')) {
            $body = $decoded;
        }
    }
}

// Auto-register user by public key
try {
    $stmt = $pdo->prepare("SELECT id FROM users WHERE public_key = ?");
    $stmt->execute([$identity]);
    $existingUser = $stmt->fetch();
    if ($existingUser) {
        $userId = $existingUser['id'];
    } else {
        $pdo->prepare("INSERT INTO users (id, public_key) VALUES (?, ?) ON DUPLICATE KEY UPDATE public_key = public_key")
            ->execute([$userId, $identity]);
    }

    // Throttled device heartbeat
    $pdo->prepare("UPDATE devices SET last_seen = CURRENT_TIMESTAMP WHERE id = ? AND (last_seen IS NULL OR last_seen < DATE_SUB(NOW(), INTERVAL 60 SECOND))")
        ->execute([$deviceId]);
} catch (PDOException $e) {
    error_log("[WebApp API] Auth DB error (user/device): " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Auth service error: " . $e->getMessage()]);
    exit;
}

// ── Router ────────────────────────────────────────────────────────────────────
try {
    switch ($route) {

        // ── Supporter status — authenticated users who've imported are supporters ──
        case 'webapp/supporter-status':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') { http_response_code(405); break; }
            $stmt = $pdo->prepare("SELECT supporter FROM users WHERE id = ?");
            $stmt->execute([$userId]);
            $row = $stmt->fetch();
            // Check if there's any imported data
            $hasData = false;
            if ($row) {
                $hasData = (bool)$row['supporter'];
            }
            echo json_encode(["supporter" => true, "needs_import" => !$hasData]);
            break;

        // ── Setup webhook secret — browser calls this before registering on questlinecli.com ──
        case 'webhook/setup':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); break; }
            $data   = json_decode($body, true);
            $secret = trim($data['secret'] ?? '');
            if (empty($secret) || strlen($secret) > 128) {
                http_response_code(400);
                echo json_encode(["error" => "Invalid secret"]);
                break;
            }
            $pdo->prepare("INSERT INTO webhook_secrets (user_id, secret) VALUES (?, ?) ON DUPLICATE KEY UPDATE secret = ?")
                ->execute([$userId, $secret, $secret]);
            echo json_encode(["status" => "ok"]);
            break;

        // ── Import — bulk insert events from questlinecli.com (initial data seed) ──
        case 'webapp/import':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); break; }
            $events = json_decode($body, true);
            if (!is_array($events)) {
                http_response_code(400);
                echo json_encode(["error" => "Expected array of events"]);
                break;
            }

            $pdo->beginTransaction();
            $stmt = $pdo->prepare(
                "INSERT IGNORE INTO sync_events (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            );
            $inserted = 0;
            foreach ($events as $e) {
                if (empty($e['id'])) continue;
                $stmt->execute([
                    $e['id'],
                    $userId,
                    $e['entity_type'],
                    $e['entity_id'],
                    $e['operation'],
                    $e['content']  ?? '',
                    $e['timestamp'] ?? date(DATE_RFC3339),
                    $e['device_id'] ?? '',
                ]);
                $inserted += $stmt->rowCount();
            }
            $pdo->commit();

            // Mark user as having imported data
            $pdo->prepare("UPDATE users SET supporter = 1 WHERE id = ?")->execute([$userId]);

            echo json_encode(["status" => "ok", "inserted" => $inserted]);
            break;

        // ── Sync push — store events in gibranlp_webappquest ─────────────────────
        case 'sync/push':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); break; }
            $entries = json_decode($body, true);
            if (!is_array($entries)) {
                http_response_code(400);
                echo json_encode(["error" => "Expected array of sync events"]);
                break;
            }

            $pdo->beginTransaction();
            $stmt = $pdo->prepare(
                "INSERT IGNORE INTO sync_events (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            );
            $inserted = 0;
            foreach ($entries as $e) {
                if (empty($e['id'])) continue;
                $eventDeviceId = $e['device_id'] ?? $deviceId ?? '';
                $stmt->execute([
                    $e['id'],
                    $userId,
                    $e['entity_type'],
                    $e['entity_id'],
                    $e['operation'],
                    $e['content'] ?? '',
                    $e['timestamp'],
                    $eventDeviceId,
                ]);
                $inserted += $stmt->rowCount();
            }
            $pdo->commit();

            // Register device if not seen
            $pdo->prepare("INSERT INTO devices (id, user_id, device_name, public_key) VALUES (?, ?, 'Questline Web', ?)
                ON DUPLICATE KEY UPDATE last_seen = CURRENT_TIMESTAMP")
                ->execute([$deviceId, $userId, $identity]);

            echo json_encode(["status" => "success", "pushed" => $inserted]);
            break;

        // ── Sync pull — return events for this user from gibranlp_webappquest ────
        case 'sync/pull':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); break; }
            $sinceSeq = isset($_GET['since_seq']) ? (int)$_GET['since_seq'] : 0;
            $stmt = $pdo->prepare(
                "SELECT id, entity_type, entity_id, operation, payload AS content,
                        created_at AS timestamp, device_id, seq
                 FROM sync_events
                 WHERE user_id = ? AND seq > ?
                 ORDER BY seq ASC LIMIT 500"
            );
            $stmt->execute([$userId, $sinceSeq]);
            echo json_encode($stmt->fetchAll());
            break;

        // ── Device register — record device so presence tracking works ──────────
        case 'devices/register':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') { http_response_code(405); break; }
            $data       = json_decode($body, true);
            $deviceName = substr($data['device_name'] ?? 'Questline Web', 0, 100);
            $pdo->prepare("INSERT INTO devices (id, user_id, device_name, public_key) VALUES (?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE device_name = ?, last_seen = CURRENT_TIMESTAMP")
                ->execute([$deviceId, $userId, $deviceName, $identity, $deviceName]);
            echo json_encode(["status" => "success"]);
            break;

        default:
            http_response_code(404);
            echo json_encode(["error" => "Not found"]);
            break;
    }
} catch (PDOException $e) {
    error_log("[WebApp API] DB error: " . $e->getMessage());
    if ($pdo->inTransaction()) $pdo->rollBack();
    http_response_code(500);
    echo json_encode(["error" => "Service temporarily unavailable"]);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function verify_ed25519($publicKeyHex, $signatureHex, $message) {
    try {
        if (!extension_loaded('sodium')) return false;
        $pub = sodium_hex2bin($publicKeyHex);
        $sig = sodium_hex2bin($signatureHex);
        if (strlen($pub) !== 32 || strlen($sig) !== 64) return false;
        return sodium_crypto_sign_verify_detached($sig, $message, $pub);
    } catch (Exception $e) { return false; }
}

function curl_post_json($url, $body) {
    $ch = curl_init($url);
    curl_setopt_array($ch, [
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_POST           => true,
        CURLOPT_POSTFIELDS     => $body,
        CURLOPT_HTTPHEADER     => ['Content-Type: application/json'],
        CURLOPT_TIMEOUT        => 8,
        CURLOPT_CONNECTTIMEOUT => 4,
    ]);
    $resp = curl_exec($ch);
    $code = curl_getinfo($ch, CURLINFO_HTTP_CODE);
    curl_close($ch);
    return ['code' => $code ?: 502, 'body' => $resp ?: '{"error":"Upstream unavailable"}'];
}

function curl_get_json($url) {
    $ch = curl_init($url);
    curl_setopt_array($ch, [
        CURLOPT_RETURNTRANSFER => true,
        CURLOPT_TIMEOUT        => 8,
        CURLOPT_CONNECTTIMEOUT => 4,
    ]);
    $resp = curl_exec($ch);
    $code = curl_getinfo($ch, CURLINFO_HTTP_CODE);
    curl_close($ch);
    return ['code' => $code ?: 502, 'body' => $resp ?: '{"error":"Upstream unavailable"}'];
}

function setup_tables($pdo) {
    // Each statement in its own exec() — PDO only runs the first statement
    // in a multi-statement batch, so we split them to ensure all tables exist.
    $pdo->exec("CREATE TABLE IF NOT EXISTS users (
        id          VARCHAR(36)  PRIMARY KEY,
        public_key  VARCHAR(64)  NOT NULL DEFAULT '',
        supporter   TINYINT(1)   NOT NULL DEFAULT 0,
        created_at  TIMESTAMP    DEFAULT CURRENT_TIMESTAMP,
        INDEX idx_pk (public_key)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");

    $pdo->exec("CREATE TABLE IF NOT EXISTS devices (
        id          VARCHAR(36)  PRIMARY KEY,
        user_id     VARCHAR(36)  NOT NULL,
        device_name VARCHAR(255) NOT NULL DEFAULT '',
        public_key  VARCHAR(64)  NOT NULL DEFAULT '',
        last_seen   TIMESTAMP    DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
        created_at  TIMESTAMP    DEFAULT CURRENT_TIMESTAMP,
        INDEX idx_user (user_id)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");

    $pdo->exec("CREATE TABLE IF NOT EXISTS sync_events (
        id          VARCHAR(36)  PRIMARY KEY,
        user_id     VARCHAR(36)  NOT NULL,
        entity_type VARCHAR(50)  NOT NULL,
        entity_id   VARCHAR(36)  NOT NULL,
        operation   VARCHAR(20)  NOT NULL,
        payload     LONGTEXT     NOT NULL DEFAULT '',
        created_at  VARCHAR(50)  NOT NULL DEFAULT '',
        device_id   VARCHAR(36)  NOT NULL DEFAULT '',
        seq         BIGINT       NOT NULL AUTO_INCREMENT,
        KEY idx_seq (seq),
        INDEX idx_user_seq (user_id, seq)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");

    $pdo->exec("CREATE TABLE IF NOT EXISTS nonces (
        user_id    VARCHAR(36)  NOT NULL,
        nonce      VARCHAR(128) NOT NULL,
        created_at TIMESTAMP    DEFAULT CURRENT_TIMESTAMP,
        PRIMARY KEY (user_id, nonce)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");

    $pdo->exec("CREATE TABLE IF NOT EXISTS webhook_secrets (
        user_id VARCHAR(36)  PRIMARY KEY,
        secret  VARCHAR(128) NOT NULL
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4");
}
?>
