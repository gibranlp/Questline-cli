<?php
// ─────────────────────────────────────────────────────────────────────────────
// api/index.php — el backend PHP que sincroniza datos y maneja dispositivos
// ─────────────────────────────────────────────────────────────────────────────

header("Content-Type: application/json; charset=UTF-8");
header("Access-Control-Allow-Origin: *");
header("Access-Control-Allow-Headers: Content-Type, X-Identity, X-User-Id, X-Device-Id, X-Timestamp, X-Nonce, X-Signature");
header("Access-Control-Allow-Methods: GET, POST, OPTIONS");
header("X-Content-Type-Options: nosniff");
header("X-Frame-Options: DENY");
header("Referrer-Policy: no-referrer");

if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') {
    exit(0);
}

// No HTTPS en localhost, pero en prod es obligatorio — no queremos datos en claro por ahí
$host = $_SERVER['HTTP_HOST'] ?? '';
$isLocal = str_contains($host, 'localhost') || str_contains($host, '127.0.0.1');
if (!$isLocal && (empty($_SERVER['HTTPS']) || $_SERVER['HTTPS'] === 'off')) {
    http_response_code(403);
    echo json_encode(["error" => "Security Error: HTTPS required"]);
    exit;
}

require_once dirname(__DIR__) . '/load_env.php';

// Credenciales del DB desde variables de entorno — nunca hardcodeadas, órale
$db_host = getenv('DB_HOST') ?: 'localhost';
$db_name = getenv('DB_NAME') ?: 'questline';
$db_user = getenv('DB_USER') ?: 'root';
$db_pass = getenv('DB_PASS') ?: '';


try {
    $pdo = new PDO("mysql:host=$db_host;charset=utf8mb4", $db_user, $db_pass, [
        PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
        PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC
    ]);
    
    // Crea la DB si no existe y la selecciona — útil en instalaciones nuevas
    $pdo->exec("CREATE DATABASE IF NOT EXISTS `$db_name` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;");
    $pdo->exec("USE `$db_name`;");
    
    // Si no hay tabla de usuarios, se auto-configura todo — chido para installs frescos
    $tableCheck = $pdo->query("SHOW TABLES LIKE 'users'")->rowCount();
    if ($tableCheck === 0) {
        setup_tables($pdo);
    }

    // Migraciones incrementales — tablas nuevas que no tenían las instalaciones viejas, no manches
    $pdo->exec("
        CREATE TABLE IF NOT EXISTS chapter_progress (
            chapter_id VARCHAR(50) PRIMARY KEY,
            completed TINYINT(1) NOT NULL DEFAULT 0,
            completed_at TIMESTAMP NULL,
            last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    $pdo->exec("
        CREATE TABLE IF NOT EXISTS chapter_objectives (
            chapter_id VARCHAR(50) NOT NULL,
            objective_type VARCHAR(50) NOT NULL,
            current_value BIGINT UNSIGNED NOT NULL DEFAULT 0,
            target_value BIGINT UNSIGNED NOT NULL,
            PRIMARY KEY (chapter_id, objective_type)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    $pdo->exec("
        CREATE TABLE IF NOT EXISTS chapter_contributions (
            id INT AUTO_INCREMENT PRIMARY KEY,
            user_id VARCHAR(36) NOT NULL,
            chapter_id VARCHAR(50) NOT NULL,
            objective_type VARCHAR(50) NOT NULL,
            total_contributed BIGINT UNSIGNED NOT NULL DEFAULT 0,
            UNIQUE KEY uq_chapter_contrib (user_id, chapter_id, objective_type)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    $pdo->exec("
        CREATE TABLE IF NOT EXISTS bug_reports (
            id INT AUTO_INCREMENT PRIMARY KEY,
            user_id VARCHAR(36) NULL,
            report_type VARCHAR(30) NOT NULL,
            description TEXT NOT NULL,
            version VARCHAR(20) NULL,
            os VARCHAR(50) NULL,
            arch VARCHAR(20) NULL,
            term VARCHAR(50) NULL,
            term_program VARCHAR(50) NULL,
            username VARCHAR(255) NULL,
            class VARCHAR(50) NULL,
            level INT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    $pdo->exec("
        CREATE TABLE IF NOT EXISTS global_chronicle (
            id VARCHAR(64) PRIMARY KEY,
            hero_name VARCHAR(100) NOT NULL DEFAULT '',
            event_type VARCHAR(50) NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            timestamp VARCHAR(50) NOT NULL DEFAULT '',
            INDEX idx_gc_timestamp (timestamp)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    // Tabla temporal para los códigos OAuth de Spotify — se autolimpian a los 5 minutos
    $pdo->exec("
        CREATE TABLE IF NOT EXISTS spotify_auth_codes (
            state VARCHAR(128) PRIMARY KEY,
            code VARCHAR(512) NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    // Migraciones: agregar columnas de stats y tabla de webhooks
    $cols = [
        'username' => 'VARCHAR(255) NULL',
        'class' => 'VARCHAR(50) NULL',
        'level' => 'INT NOT NULL DEFAULT 1',
        'streak' => 'INT NOT NULL DEFAULT 0',
        'xp' => 'INT NOT NULL DEFAULT 0'
    ];
    foreach ($cols as $col => $type) {
        try {
            $pdo->exec("ALTER TABLE users ADD COLUMN $col $type");
        } catch (PDOException $e) {
            if (strpos($e->getMessage(), '1060') === false) { throw $e; }
        }
    }

    $pdo->exec("
        CREATE TABLE IF NOT EXISTS webhooks (
            id INT AUTO_INCREMENT PRIMARY KEY,
            user_id VARCHAR(36) NOT NULL,
            url VARCHAR(512) NOT NULL,
            events VARCHAR(255) NOT NULL,
            secret VARCHAR(64) NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    ");
    // Migraciones: agregar device_id y seq a sync_events para filtrar propios eventos y cursor incremental
    try {
        $pdo->exec("ALTER TABLE sync_events ADD COLUMN device_id VARCHAR(36) NOT NULL DEFAULT ''");
        $pdo->exec("ALTER TABLE sync_events ADD INDEX idx_sync_events_device (user_id, device_id)");
    } catch (PDOException $e) {
        if (strpos($e->getMessage(), '1060') === false && strpos($e->getMessage(), '1061') === false) { throw $e; }
    }
    // All ALTER TABLE migrations are wrapped to silently ignore both
    // "column already exists" (1060) and "permission denied" (1142) errors
    // so a failed migration never brings down unrelated routes.
    foreach ([
        "ALTER TABLE sync_events ADD COLUMN seq BIGINT NOT NULL AUTO_INCREMENT, ADD KEY idx_seq (seq)",
        "ALTER TABLE sync_events ADD INDEX idx_sync_events_seq (user_id, seq)",
        "ALTER TABLE users ADD COLUMN supporter TINYINT(1) NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN email VARCHAR(255) NULL",
        "ALTER TABLE access_codes ADD COLUMN linked_user_id VARCHAR(36) NULL",
        "ALTER TABLE devices ADD COLUMN public_key VARCHAR(64) NULL",
        "ALTER TABLE webhooks ADD UNIQUE KEY uq_user_url (user_id, url(255))",
    ] as $_migration) {
        try { $pdo->exec($_migration); } catch (PDOException $e) { /* non-fatal */ }
    }
    unset($_migration);
    foreach ([
        "CREATE TABLE IF NOT EXISTS access_codes (
            code VARCHAR(64) PRIMARY KEY,
            label VARCHAR(255) NULL,
            created_by VARCHAR(100) NOT NULL DEFAULT 'admin',
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            redeemed_by_user_id VARCHAR(36) NULL,
            redeemed_at TIMESTAMP NULL
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
        "CREATE TABLE IF NOT EXISTS pending_supporters (
            email VARCHAR(255) PRIMARY KEY,
            granted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
        "CREATE TABLE IF NOT EXISTS webapp_accounts (
            id INT AUTO_INCREMENT PRIMARY KEY,
            user_id VARCHAR(36) NOT NULL,
            username VARCHAR(50) NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            public_key VARCHAR(64) NOT NULL,
            encrypted_key_blob MEDIUMTEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE KEY uq_username (username),
            INDEX idx_user_id (user_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
    ] as $_createSql) {
        try { $pdo->exec($_createSql); } catch (PDOException $e) { /* non-fatal */ }
    }
    unset($_createSql);
} catch (PDOException $e) {
    error_log("[Questline API] DB connection failed: " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Service temporarily unavailable"]);
    exit;
}

// ── Parse Route early for public bypass ────
$requestUri = $_SERVER['REQUEST_URI'] ?? '';
$apiPath = parse_url($requestUri, PHP_URL_PATH);
$pathSegments = explode('/api/', $apiPath);
$route = isset($pathSegments[1]) ? rtrim($pathSegments[1], '/') : '';
if (str_starts_with($route, 'index.php/')) {
    $route = substr($route, 10);
} else if ($route === 'index.php') {
    $route = '';
}
if (empty($route)) {
    $route = $_GET['route'] ?? '';
}

if ($route === 'public/chapter_stats') {
    $chapter_id = $_GET['chapter_id'] ?? 'chapter_one';
    $chapter_id = preg_replace('/[^a-z0-9_]/', '', strtolower($chapter_id));
    if (empty($chapter_id)) { $chapter_id = 'chapter_one'; }
    $stmt = $pdo->prepare("SELECT completed, completed_at FROM chapter_progress WHERE chapter_id = ?");
    $stmt->execute([$chapter_id]);
    $progress = $stmt->fetch() ?: ["completed" => 0, "completed_at" => null];
    $stmt = $pdo->prepare("SELECT objective_type, current_value, target_value FROM chapter_objectives WHERE chapter_id = ?");
    $stmt->execute([$chapter_id]);
    $objectives = $stmt->fetchAll();
    echo json_encode(["status" => "success", "chapter_id" => $chapter_id, "completed" => $progress['completed'], "completed_at" => $progress['completed_at'], "objectives" => $objectives]);
    exit;
}

if ($route === 'public/profile') {
    $username = $_GET['username'] ?? '';
    $username = trim($username);
    if (empty($username)) {
        http_response_code(400);
        echo json_encode(["error" => "Username parameter is required"]);
        exit;
    }
    $stmt = $pdo->prepare("SELECT username, class, level, streak, xp FROM users WHERE username = ?");
    $stmt->execute([$username]);
    $profile = $stmt->fetch();
    if (!$profile) {
        http_response_code(404);
        echo json_encode(["error" => "User profile not found"]);
        exit;
    }
    echo json_encode(["status" => "success", "profile" => $profile]);
    exit;
}

if ($route === 'public/webhook/test') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
        http_response_code(405);
        echo json_encode(["error" => "Method not allowed"]);
        exit;
    }
    echo json_encode(["status" => "success", "message" => "Webhook test successful"]);
    exit;
}

// ── Spotify config pública — la app jala el client_id del servidor, no del usuario ──
if ($route === 'spotify/config') {
    $client_id = getenv('SPOTIFY_CLIENT_ID') ?: '';
    if (empty($client_id)) {
        http_response_code(503);
        echo json_encode(["error" => "Spotify not configured on this server"]);
        exit;
    }
    $redirect_uri = (isset($_SERVER['HTTPS']) && $_SERVER['HTTPS'] !== 'off' ? 'https' : 'http')
        . '://' . ($_SERVER['HTTP_HOST'] ?? 'questlinecli.com')
        . '/api/spotify/callback';
    echo json_encode(["client_id" => $client_id, "redirect_uri" => $redirect_uri]);
    exit;
}

// ── Spotify OAuth callback — Spotify redirige al navegador aquí con el código ──
// No requiere auth de Questline; la seguridad viene del parámetro `state` único por sesión
if ($route === 'spotify/callback') {
    $code  = trim($_GET['code']  ?? '');
    $state = trim($_GET['state'] ?? '');
    $error = trim($_GET['error'] ?? '');

    // Spotify mandó error (e.g. acceso denegado por el usuario)
    if (!empty($error)) {
        header('Content-Type: text/html; charset=UTF-8');
        echo '<!doctype html><html><head><meta charset="UTF-8"><title>Spotify — Questline</title>'
           . '<style>body{font-family:system-ui,sans-serif;background:#0d1117;color:#e6edf3;display:flex;'
           . 'flex-direction:column;align-items:center;justify-content:center;height:100vh;margin:0}'
           . 'h2{color:#f85149}p{color:#8b949e}</style></head><body>'
           . '<h2>Authorization cancelled</h2>'
           . '<p>You denied access. You can close this tab and try again in Questline.</p>'
           . '</body></html>';
        exit;
    }

    if (empty($code) || empty($state)) {
        http_response_code(400);
        header('Content-Type: text/html; charset=UTF-8');
        echo '<!doctype html><html><body>Bad request — missing code or state.</body></html>';
        exit;
    }

    // Sanitización básica — state y code solo llevan caracteres URL-safe
    if (!preg_match('/^[A-Za-z0-9\-_]{4,128}$/', $state) ||
        !preg_match('/^[A-Za-z0-9\-_\.]{10,512}$/', $code)) {
        http_response_code(400);
        header('Content-Type: text/html; charset=UTF-8');
        echo '<!doctype html><html><body>Invalid parameters.</body></html>';
        exit;
    }

    // Limpia códigos expirados (>5 min) antes de insertar
    $pdo->exec("DELETE FROM spotify_auth_codes WHERE created_at < DATE_SUB(NOW(), INTERVAL 5 MINUTE)");

    $stmt = $pdo->prepare(
        "INSERT INTO spotify_auth_codes (state, code) VALUES (?, ?)
         ON DUPLICATE KEY UPDATE code = VALUES(code), created_at = CURRENT_TIMESTAMP"
    );
    $stmt->execute([$state, $code]);

    header('Content-Type: text/html; charset=UTF-8');
    echo '<!doctype html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Spotify Connected — Questline</title>
  <style>
    *{box-sizing:border-box;margin:0;padding:0}
    body{font-family:system-ui,-apple-system,sans-serif;background:#0d1117;color:#e6edf3;
         display:flex;flex-direction:column;align-items:center;justify-content:center;
         min-height:100vh;gap:1.5rem;padding:2rem;text-align:center}
    .icon{font-size:3rem}
    h1{font-size:1.5rem;font-weight:700;color:#1db954}
    p{color:#8b949e;max-width:380px;line-height:1.6}
    .badge{background:#161b22;border:1px solid #30363d;border-radius:8px;
           padding:.6rem 1.2rem;font-size:.85rem;color:#58a6ff}
  </style>
</head>
<body>
  <div class="icon">⚔️</div>
  <h1>Spotify Connected!</h1>
  <p>Questline is now linked to your Spotify account. You can close this tab and return to the terminal.</p>
  <div class="badge">Returning you to the Realm...</div>
  <script>setTimeout(()=>window.close(),3000)</script>
</body>
</html>';
    exit;
}

// ── Spotify token poll — la app consulta esto cada segundo hasta recibir el código ──
if ($route === 'spotify/token') {
    $state = trim($_GET['state'] ?? '');

    if (empty($state) || !preg_match('/^[A-Za-z0-9\-_]{4,128}$/', $state)) {
        http_response_code(400);
        echo json_encode(["error" => "Invalid state parameter"]);
        exit;
    }

    // Limpia expirados de paso
    $pdo->exec("DELETE FROM spotify_auth_codes WHERE created_at < DATE_SUB(NOW(), INTERVAL 5 MINUTE)");

    $stmt = $pdo->prepare("SELECT code FROM spotify_auth_codes WHERE state = ?");
    $stmt->execute([$state]);
    $row = $stmt->fetch();

    if (!$row) {
        http_response_code(404);
        echo json_encode(["status" => "pending"]);
        exit;
    }

    // Una sola entrega — borra el código inmediatamente para que no se reutilice
    $pdo->prepare("DELETE FROM spotify_auth_codes WHERE state = ?")->execute([$state]);

    echo json_encode(["status" => "ok", "code" => $row['code']]);
    exit;
}

// ── Webapp registration — ruta pública, no requiere auth criptográfica ─────────
if ($route === 'webapp/register') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
        http_response_code(405);
        echo json_encode(["error" => "Method not allowed"]);
        exit;
    }
    $rawBody = file_get_contents('php://input');
    $data = json_decode($rawBody, true);
    if (!$data) {
        http_response_code(400);
        echo json_encode(["error" => "Invalid request body"]);
        exit;
    }

    $regCode        = trim($data['access_code']       ?? '');
    $regUsername    = trim($data['username']           ?? '');
    $regPassword    = $data['password']                ?? '';
    $regUserId      = trim($data['user_id']            ?? '');
    $regPubKey      = trim($data['public_key']         ?? '');
    $regDevId       = trim($data['device_id']          ?? '');
    $regDevName     = substr(trim($data['device_name'] ?? 'Questline Web'), 0, 100);
    $regKeyBlob     = $data['encrypted_key_blob']      ?? '';

    // Validate required fields
    if (empty($regCode) || empty($regUsername) || empty($regPassword) ||
        strlen($regPubKey) !== 64 || !ctype_xdigit($regPubKey) ||
        empty($regKeyBlob)) {
        http_response_code(400);
        echo json_encode(["error" => "Missing required fields"]);
        exit;
    }
    if (!preg_match('/^[a-zA-Z0-9_]{1,50}$/', $regUsername)) {
        http_response_code(400);
        echo json_encode(["error" => "Username must be 1-50 alphanumeric characters or underscores"]);
        exit;
    }
    if (strlen($regPassword) < 8) {
        http_response_code(400);
        echo json_encode(["error" => "Password must be at least 8 characters"]);
        exit;
    }

    // Validate access code and check for linked_user_id
    $stmt = $pdo->prepare("SELECT code, linked_user_id FROM access_codes WHERE code = ? AND (redeemed_by_user_id IS NULL OR redeemed_by_user_id = '')");
    $stmt->execute([$regCode]);
    $codeRow = $stmt->fetch();
    if (!$codeRow) {
        http_response_code(403);
        echo json_encode(["error" => "Access code is invalid or already used"]);
        exit;
    }

    // Check username availability
    $stmt = $pdo->prepare("SELECT id FROM webapp_accounts WHERE username = ?");
    $stmt->execute([$regUsername]);
    if ($stmt->fetch()) {
        http_response_code(409);
        echo json_encode(["error" => "Username already taken"]);
        exit;
    }

    // Determine final user_id: prefer linked_user_id from code (connects to existing CLI account)
    $finalUserId = !empty($codeRow['linked_user_id']) ? $codeRow['linked_user_id'] : $regUserId;
    if (empty($finalUserId) || !preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i', $finalUserId)) {
        $finalUserId = strtolower(bin2hex(random_bytes(4)) . '-' . bin2hex(random_bytes(2)) . '-4' . substr(bin2hex(random_bytes(2)), 1) . '-' . dechex(8 + rand(0, 3)) . substr(bin2hex(random_bytes(2)), 1) . '-' . bin2hex(random_bytes(6)));
    }

    $passwordHash = password_hash($regPassword, PASSWORD_BCRYPT, ['cost' => 12]);

    // Upsert user record — if linked_user_id, just set supporter flag; else create new
    $stmt = $pdo->prepare("INSERT INTO users (id, public_key, supporter) VALUES (?, ?, 1) ON DUPLICATE KEY UPDATE supporter = 1");
    $stmt->execute([$finalUserId, $regPubKey]);

    // Create webapp account
    $stmt = $pdo->prepare("INSERT INTO webapp_accounts (user_id, username, password_hash, public_key, encrypted_key_blob) VALUES (?, ?, ?, ?, ?)");
    $stmt->execute([$finalUserId, $regUsername, $passwordHash, $regPubKey, $regKeyBlob]);

    // Register webapp device with its public key so Paso 3 can resolve user_id from it
    if (!empty($regDevId) && preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i', $regDevId)) {
        $stmt = $pdo->prepare("INSERT INTO devices (id, user_id, device_name, public_key) VALUES (?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE device_name = VALUES(device_name), public_key = VALUES(public_key), last_seen = CURRENT_TIMESTAMP");
        $stmt->execute([$regDevId, $finalUserId, $regDevName, $regPubKey]);
    }

    // Mark code redeemed
    $stmt = $pdo->prepare("UPDATE access_codes SET redeemed_by_user_id = ?, redeemed_at = CURRENT_TIMESTAMP WHERE code = ?");
    $stmt->execute([$finalUserId, $regCode]);

    echo json_encode(["status" => "success", "user_id" => $finalUserId]);
    exit;
}

// ── Webapp check-code — validates an access code without consuming it ──────────
if ($route === 'webapp/check-code') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
        http_response_code(405);
        echo json_encode(["error" => "Method not allowed"]);
        exit;
    }
    $rawBody = file_get_contents('php://input');
    $data = json_decode($rawBody, true);
    $code = trim($data['access_code'] ?? '');

    if (empty($code)) {
        http_response_code(400);
        echo json_encode(["error" => "Access code required"]);
        exit;
    }

    $stmt = $pdo->prepare("SELECT linked_user_id FROM access_codes WHERE code = ? AND (redeemed_by_user_id IS NULL OR redeemed_by_user_id = '')");
    $stmt->execute([$code]);
    $row = $stmt->fetch();

    if (!$row) {
        echo json_encode(["valid" => false]);
    } else {
        echo json_encode(["valid" => true, "linked" => !empty($row['linked_user_id'])]);
    }
    exit;
}

// ── Webapp login — ruta pública, devuelve el blob cifrado del key ──────────────
if ($route === 'webapp/login') {
    if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
        http_response_code(405);
        echo json_encode(["error" => "Method not allowed"]);
        exit;
    }
    $rawBody = file_get_contents('php://input');
    $data = json_decode($rawBody, true);
    if (!$data) {
        http_response_code(400);
        echo json_encode(["error" => "Invalid request body"]);
        exit;
    }

    $loginUsername = trim($data['username'] ?? '');
    $loginPassword = $data['password'] ?? '';

    if (empty($loginUsername) || empty($loginPassword)) {
        http_response_code(400);
        echo json_encode(["error" => "Username and password are required"]);
        exit;
    }

    $stmt = $pdo->prepare("SELECT wa.*, u.supporter FROM webapp_accounts wa JOIN users u ON u.id = wa.user_id WHERE wa.username = ?");
    $stmt->execute([$loginUsername]);
    $account = $stmt->fetch();

    if (!$account || !password_verify($loginPassword, $account['password_hash'])) {
        http_response_code(401);
        echo json_encode(["error" => "Invalid username or password"]);
        exit;
    }

    if (!$account['supporter']) {
        http_response_code(403);
        echo json_encode(["error" => "Access revoked. Contact support."]);
        exit;
    }

    echo json_encode([
        "status"             => "success",
        "user_id"            => $account['user_id'],
        "public_key"         => $account['public_key'],
        "encrypted_key_blob" => $account['encrypted_key_blob'],
    ]);
    exit;
}

if ($route === 'webapp/check-email') {
    $checkEmail = trim($_GET['email'] ?? '');
    if (!filter_var($checkEmail, FILTER_VALIDATE_EMAIL)) {
        echo json_encode(["pre_authorized" => false]);
        exit;
    }
    $stmt = $pdo->prepare("SELECT email FROM pending_supporters WHERE email = ?");
    $stmt->execute([$checkEmail]);
    echo json_encode(["pre_authorized" => (bool)$stmt->fetch()]);
    exit;
}

// ── Paso 1: Agarrar los headers de autenticación — sin estos no pasa nadie ────
$headers = getallheaders();
$userId = $headers['X-User-Id'] ?? $headers['x-user-id'] ?? null;
$identity = $headers['X-Identity'] ?? $headers['x-identity'] ?? null;
$deviceId = $headers['X-Device-Id'] ?? $headers['x-device-id'] ?? null;
$timestamp = $headers['X-Timestamp'] ?? $headers['x-timestamp'] ?? null;
$nonce = $headers['X-Nonce'] ?? $headers['x-nonce'] ?? null;
$signature = $headers['X-Signature'] ?? $headers['x-signature'] ?? null;

if (!$userId || !$identity || !$deviceId || !$timestamp || !$nonce || !$signature) {
    log_api_event($pdo, null, null, 'AUTH_FAILURE', 'Missing authentication headers');
    http_response_code(400);
    echo json_encode(["error" => "Security Error: Missing cryptographic authentication headers"]);
    exit;
}

// Validar formatos antes de tocar la DB o hacer crypto — fail fast, pues
if (!preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i', $userId)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid request format"]);
    exit;
}
if (!preg_match('/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i', $deviceId)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid request format"]);
    exit;
}
// Identity es la llave pública Ed25519 en hex — 64 chars exactos
if (strlen($identity) !== 64 || !ctype_xdigit($identity)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid request format"]);
    exit;
}
// La firma son 64 bytes = 128 hex chars — si no cuadra, alguien está haciendo cosas raras
if (strlen($signature) !== 128 || !ctype_xdigit($signature)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid request format"]);
    exit;
}
// Nonce: puede venir como UUID o hex crudo — flexible para no romper clientes viejos
$nonceStripped = str_replace('-', '', $nonce);
$nonceLen = strlen($nonceStripped);
if ($nonceLen < 16 || $nonceLen > 128 || !ctype_xdigit($nonceStripped)) {
    http_response_code(400);
    echo json_encode(["error" => "Invalid request format"]);
    exit;
}

// ── Paso 2: Ventana de 5 minutos — requests viejos se rechazan de plano ───────
$requestTime = strtotime($timestamp);
if (!$requestTime || abs(time() - $requestTime) > 300) {
    log_api_event($pdo, $userId, $deviceId, 'AUTH_FAILURE', "Timestamp expired: $timestamp (Server: " . date(DATE_RFC3339) . ")");
    http_response_code(401);
    echo json_encode(["error" => "Security Error: Request timestamp is out of range. Check local system time."]);
    exit;
}

// ── Paso 3: Registra al héroe en su primera visita o verifica que la llave cuadre
try {
    // Primero, buscamos si ya existe la llave pública (identity)
    $stmt = $pdo->prepare("SELECT id FROM users WHERE public_key = ?");
    $stmt->execute([$identity]);
    $existingUser = $stmt->fetch();

    if ($existingUser) {
        // Si ya existe la llave pública, forzamos el user ID al que está registrado en la base de datos.
        // Esto soluciona el problema de tener el mismo par de llaves en múltiples PCs con diferentes UUIDs.
        $userId = $existingUser['id'];
    } else {
        // Check devices table for per-device keys (webapp uses its own keypair per account)
        $stmt = $pdo->prepare("SELECT user_id FROM devices WHERE public_key = ? LIMIT 1");
        $stmt->execute([$identity]);
        $deviceByKey = $stmt->fetch();
        if ($deviceByKey) {
            $userId = $deviceByKey['user_id'];
        } else {
        // Si la llave pública no existe, validamos el user ID enviado por el cliente
        $stmt = $pdo->prepare("SELECT public_key FROM users WHERE id = ?");
        $stmt->execute([$userId]);
        $userById = $stmt->fetch();

        if ($userById) {
            // Si el user ID ya está tomado por otra llave pública, esto es un error grave de conflicto
            log_api_event($pdo, $userId, $deviceId, 'AUTH_FAILURE', "User ID already taken by different public key");
            http_response_code(403);
            echo json_encode(["error" => "Security Error: User ID already taken by different public key"]);
            exit;
        } else {
            // Registramos al usuario nuevo con la combinación dada
            $stmt = $pdo->prepare("INSERT INTO users (id, public_key) VALUES (?, ?)");
            $stmt->execute([$userId, $identity]);
        }
        } // closes: else { // deviceByKey not found
    }
} catch (PDOException $e) {
    error_log("[Questline API] Auth DB error (step 3): " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Service temporarily unavailable"]);
    exit;
}

// ── Paso 4: Nonce único por request — previene replay attacks, qué rollo sería sin esto
try {
    $stmt = $pdo->prepare("SELECT 1 FROM nonces WHERE user_id = ? AND nonce = ?");
    $stmt->execute([$userId, $nonce]);
    if ($stmt->fetch()) {
        log_api_event($pdo, $userId, $deviceId, 'AUTH_FAILURE', "Replay attack detected: used nonce $nonce");
        http_response_code(401);
        echo json_encode(["error" => "Security Error: Replay attack detected (nonce used)"]);
        exit;
    }

    // Guardar el nonce para que no se pueda reusar
    $stmt = $pdo->prepare("INSERT INTO nonces (user_id, nonce) VALUES (?, ?)");
    $stmt->execute([$userId, $nonce]);

    // Limpieza probabilística — 1% de chance, no queremos hacer DELETE en cada request
    if (mt_rand(0, 99) === 0) {
        $pdo->exec("DELETE FROM nonces WHERE created_at < DATE_SUB(NOW(), INTERVAL 10 MINUTE)");
    }
} catch (PDOException $e) {
    error_log("[Questline API] Auth DB error (step 4): " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Service temporarily unavailable"]);
    exit;
}

// ── Paso 5: Verificar la firma Ed25519 — si esto falla, el request muere aquí ──
$body = file_get_contents('php://input');
$messageToSign = $timestamp . '.' . $nonce . '.' . $body;

if (!verify_ed25519_signature($identity, $signature, $messageToSign)) {
    log_api_event($pdo, $userId, $deviceId, 'AUTH_FAILURE', "Invalid signature signature: $signature");
    http_response_code(401);
    echo json_encode(["error" => "Security Error: Cryptographic signature verification failed"]);
    exit;
}

// Algunos WAFs bloquean JSON crudo — si el body viene en base64 lo decodificamos
if ($_SERVER['REQUEST_METHOD'] === 'POST' && !empty($body)) {
    $firstChar = $body[0];
    if ($firstChar !== '{' && $firstChar !== '[') {
        $decoded = base64_decode($body, true); // strict mode — rejects non-base64 chars
        if ($decoded !== false && ($decoded[0] === '{' || $decoded[0] === '[')) {
            $body = $decoded;
        }
    }
}


// ── Paso 6: Rate limiting básico — 100 req/min, Redis sería lo ideal pero MySQL jala ──
try {
    $stmt = $pdo->prepare("SELECT COUNT(*) FROM nonces WHERE user_id = ? AND created_at > SUBDATE(NOW(), INTERVAL 1 MINUTE)");
    $stmt->execute([$userId]);
    $requestCount = $stmt->fetchColumn();
    if ($requestCount > 100) {
        log_api_event($pdo, $userId, $deviceId, 'API_ERROR', 'Rate limit exceeded');
        http_response_code(429);
        echo json_encode(["error" => "Rate limit exceeded. Try again in a minute."]);
        exit;
    }
} catch (PDOException $e) {
    // Ignore rate limit DB failures to keep functioning
}

// ── Paso 7: Actualizar last_seen del device — throttleado a 1 vez/min para no saturar la DB ──
try {
    $stmt = $pdo->prepare("UPDATE devices SET last_seen = CURRENT_TIMESTAMP WHERE id = ? AND (last_seen IS NULL OR last_seen < DATE_SUB(NOW(), INTERVAL 60 SECOND))");
    $stmt->execute([$deviceId]);
} catch (PDOException $e) {
    // Non-fatal: continue even if heartbeat update fails
}

// ── Paso 8: Router — detecta la ruta de varias formas porque los servers son rarísimos ──
$requestUri = $_SERVER['REQUEST_URI'];
// Extraer sub-path relativo a /api
$apiPath = parse_url($requestUri, PHP_URL_PATH);
$pathSegments = explode('/api/', $apiPath);
$route = isset($pathSegments[1]) ? rtrim($pathSegments[1], '/') : '';

// Strip index.php/ prefix if present (e.g., index.php/devices)
if (str_starts_with($route, 'index.php/')) {
    $route = substr($route, 10);
} else if ($route === 'index.php') {
    $route = '';
}

// Fallback to query parameter
if (empty($route)) {
    $route = $_GET['route'] ?? '';
}

// Fallback to custom header
if (empty($route)) {
    $route = $headers['X-Route'] ?? $headers['x-route'] ?? '';
}

try {
    switch ($route) {
        // ── Registro y listado de dispositivos ────────────────────────────────────
        case 'devices/register':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $deviceName = $data['device_name'] ?? 'Unknown Device';
            $realUsername = $data['username'] ?? null;

            $stmt = $pdo->prepare("INSERT INTO devices (id, user_id, device_name) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE device_name = ?, last_seen = CURRENT_TIMESTAMP");
            $stmt->execute([$deviceId, $userId, $deviceName, $deviceName]);

            // Guardar username y stats en la tabla users
            $class = $data['class'] ?? null;
            $level = isset($data['level']) ? intval($data['level']) : null;
            $streak = isset($data['streak']) ? intval($data['streak']) : null;
            $xp = isset($data['xp']) ? intval($data['xp']) : null;

            $updateFields = [];
            $params = [];

            if ($realUsername && trim($realUsername) !== '') {
                $updateFields[] = "username = ?";
                $params[] = $realUsername;

                // Actualizar en project_members también
                $stmt = $pdo->prepare("UPDATE project_members SET user_username = ? WHERE user_identity = ?");
                $stmt->execute([$realUsername, $identity]);
            }
            if ($class !== null) {
                $updateFields[] = "class = ?";
                $params[] = $class;
            }
            if ($level !== null) {
                $updateFields[] = "level = ?";
                $params[] = $level;
            }
            if ($streak !== null) {
                $updateFields[] = "streak = ?";
                $params[] = $streak;
            }
            if ($xp !== null) {
                $updateFields[] = "xp = ?";
                $params[] = $xp;
            }

            if (!empty($updateFields)) {
                $params[] = $userId;
                $sql = "UPDATE users SET " . implode(", ", $updateFields) . " WHERE id = ?";
                $stmt = $pdo->prepare($sql);
                $stmt->execute($params);
            }

            echo json_encode(["status" => "success", "message" => "Device registered/updated successfully"]);
            break;

        case 'webhooks/register':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $url = $data['url'] ?? null;
            $events = $data['events'] ?? '*';
            $secret = $data['secret'] ?? null;

            if (!$url || filter_var($url, FILTER_VALIDATE_URL) === false) {
                http_response_code(400);
                echo json_encode(["error" => "A valid webhook URL is required"]);
                exit;
            }

            $stmt = $pdo->prepare(
                "INSERT INTO webhooks (user_id, url, events, secret) VALUES (?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE events = VALUES(events), secret = VALUES(secret)"
            );
            $stmt->execute([$userId, $url, $events, $secret]);
            echo json_encode(["status" => "success", "message" => "Webhook registered successfully"]);
            break;

        case 'webhooks/list':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $stmt = $pdo->prepare("SELECT id, url, events, created_at FROM webhooks WHERE user_id = ? ORDER BY created_at DESC");
            $stmt->execute([$userId]);
            $webhooks = $stmt->fetchAll();
            echo json_encode($webhooks);
            break;

        case 'webhooks/delete':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $webhookId = $data['id'] ?? null;
            if (!$webhookId) {
                http_response_code(400);
                echo json_encode(["error" => "Webhook ID is required"]);
                exit;
            }
            $stmt = $pdo->prepare("DELETE FROM webhooks WHERE id = ? AND user_id = ?");
            $stmt->execute([$webhookId, $userId]);
            echo json_encode(["status" => "success", "message" => "Webhook deleted successfully"]);
            break;
            
        case 'devices':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $stmt = $pdo->prepare("SELECT id, device_name, created_at, last_seen FROM devices WHERE user_id = ? ORDER BY last_seen DESC");
            $stmt->execute([$userId]);
            $devices = $stmt->fetchAll();
            echo json_encode($devices);
            break;
            
        // ── Sync push/pull — el núcleo de la replicación de datos entre dispositivos ──
        case 'sync/push':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $entries = json_decode($body, true);
            if (!is_array($entries)) {
                http_response_code(400);
                echo json_encode(["error" => "Payload must be a JSON array of sync events"]);
                exit;
            }
            
            $inserted = 0;
            $pdo->beginTransaction();
            $stmt = $pdo->prepare("INSERT IGNORE INTO sync_events (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)");

            // Prepared statements listos para replicar a los otros miembros del proyecto
            $memberStmt = $pdo->prepare("SELECT user_identity FROM project_members WHERE project_id = ? AND user_identity != ?");
            $userIdStmt = $pdo->prepare("SELECT id FROM users WHERE public_key = ?");
            $replicateInsertStmt = $pdo->prepare("INSERT IGNORE INTO sync_events (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?)");
            $entityProjectStmt = $pdo->prepare("SELECT payload FROM sync_events WHERE user_id = ? AND entity_type = ? AND entity_id = ? AND payload != '' ORDER BY seq DESC LIMIT 1");

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
                    $eventDeviceId
                ]);
                $inserted += $stmt->rowCount();

                // Disparar webhooks para todos los entity types — incluyendo al webapp mirror
                trigger_webhooks($pdo, $userId, $e['entity_type'], $e['entity_id'] ?? '', $e['operation'], $e['content'] ?? '', $e['id'] ?? '', $e['timestamp'] ?? date(DATE_RFC3339));

                // Si el evento pertenece a un proyecto compartido, replicarlo a los compañeros
                $projectId = null;
                if ($e['entity_type'] === 'project') {
                    $projectId = $e['entity_id'];
                } elseif (is_project_scoped_sync_type($e['entity_type'])) {
                    $projectId = extract_project_id_from_payload($e['content'] ?? '');
                }

                // Deletes usually arrive after the local row is gone, so their payload can be empty.
                // Recover the project from the last known event for this entity or shared deletes vanish.
                if (!$projectId && is_project_scoped_sync_type($e['entity_type'])) {
                    $entityProjectStmt->execute([$userId, $e['entity_type'], $e['entity_id']]);
                    $projectId = extract_project_id_from_payload($entityProjectStmt->fetchColumn() ?: '');
                }

                if ($projectId) {
                    $memberStmt->execute([$projectId, $identity]);
                    $members = $memberStmt->fetchAll(PDO::FETCH_COLUMN);
                    foreach ($members as $memberPubKey) {
                        $userIdStmt->execute([$memberPubKey]);
                        $targetUserId = $userIdStmt->fetchColumn();
                        if ($targetUserId) {
                            $replicatedId = md5($e['id'] . $targetUserId);
                            $replicateInsertStmt->execute([
                                $replicatedId,
                                $targetUserId,
                                $e['entity_type'],
                                $e['entity_id'],
                                $e['operation'],
                                $e['content'] ?? '',
                                $e['timestamp'],
                                $eventDeviceId
                            ]);
                        }
                    }
                }
            }
            $pdo->commit();
            echo json_encode(["status" => "success", "pushed" => $inserted]);
            break;
            
        case 'sync/pull':
        case 'sync/full':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            // Filtramos propios eventos y aplicamos cursor incremental para no descargar todo en cada sync
            $pullDeviceId = $deviceId ?? '';
            $sinceSeq = isset($_GET['since_seq']) ? (int)$_GET['since_seq'] : 0;
            $stmt = $pdo->prepare("SELECT id, entity_type, entity_id, operation, payload as content, created_at as timestamp, device_id, seq FROM sync_events WHERE user_id = ? AND device_id != ? AND seq > ? ORDER BY seq ASC LIMIT 500");
            $stmt->execute([$userId, $pullDeviceId, $sinceSeq]);
            $events = $stmt->fetchAll();
            echo json_encode($events);
            break;

        case 'sync/head':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $stmt = $pdo->prepare("SELECT COALESCE(MAX(seq), 0) AS seq FROM sync_events WHERE user_id = ?");
            $stmt->execute([$userId]);
            echo json_encode(["seq" => (int)$stmt->fetchColumn()]);
            break;
            
        // ── Invitaciones a proyectos compartidos ──────────────────────────────────
        case 'invite':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $projId = $data['project_id'] ?? null;
            $projName = $data['project_name'] ?? null;
            $inviteeIdentity = $data['invitee_identity'] ?? null;
            $role = $data['role'] ?? 'Companion';
            
            if (!$projId || !$projName || !$inviteeIdentity) {
                http_response_code(400);
                echo json_encode(["error" => "Missing invitation parameters"]);
                exit;
            }
            if (strlen($inviteeIdentity) !== 64 || !ctype_xdigit($inviteeIdentity)) {
                http_response_code(400);
                echo json_encode(["error" => "Invalid invitee identity"]);
                exit;
            }
            
            // Buscar el nombre del que invita — si no tiene device name, 'Fellow Companion' pues
            $stmt = $pdo->prepare("SELECT device_name FROM devices WHERE id = ?");
            $stmt->execute([$deviceId]);
            $inviterName = $stmt->fetchColumn() ?: 'Fellow Companion';
            
            $inviteId = bin2hex(random_bytes(16));
            $stmt = $pdo->prepare("INSERT INTO project_invitations (id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, 'Pending', ?)");
            $stmt->execute([
                $inviteId,
                $projId,
                $projName,
                $identity,
                $inviterName,
                $inviteeIdentity,
                $role,
                date(DATE_RFC3339)
            ]);
            
            log_api_event($pdo, $userId, $deviceId, 'INVITATION', "Invited user $inviteeIdentity to join project $projName");
            echo json_encode(["status" => "success", "invite_id" => $inviteId]);
            break;
            
        case 'accept':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $inviteId = $data['invite_id'] ?? null;
            if (!$inviteId) {
                http_response_code(400);
                echo json_encode(["error" => "Missing invite_id parameter"]);
                exit;
            }
            
            // Buscar la invitación pendiente — solo acepta si sigue en estado Pending
            $stmt = $pdo->prepare("SELECT * FROM project_invitations WHERE id = ? AND status = 'Pending'");
            $stmt->execute([$inviteId]);
            $invite = $stmt->fetch();
            
            if (!$invite) {
                http_response_code(404);
                echo json_encode(["error" => "Pending invitation not found"]);
                exit;
            }
            
            // Marcar la invitación como aceptada
            $stmt = $pdo->prepare("UPDATE project_invitations SET status = 'Accepted' WHERE id = ?");
            $stmt->execute([$inviteId]);
            
            // Registrar ambos en project_members — el que invitó y el que aceptó
            $stmt = $pdo->prepare("INSERT INTO project_members (project_id, user_identity, user_username, role) VALUES (?, ?, ?, ?) ON DUPLICATE KEY UPDATE user_username = ?, role = ?");

            // El que invitó es Owner — el que acepta entra con el rol que le dieron
            $stmt->execute([$invite['project_id'], $invite['inviter_identity'], $invite['inviter_username'], 'Owner', $invite['inviter_username'], 'Owner']);

            // Username del invitado viene en el body — si no, se pone el nombre del rol
            $inviteeUsername = $data['username'] ?? null;
            if (!$inviteeUsername || trim($inviteeUsername) === '') {
                $inviteeUsername = $invite['role'] . ' Companion';
            }
            $stmt->execute([$invite['project_id'], $identity, $inviteeUsername, $invite['role'], $inviteeUsername, $invite['role']]);

            // El compañero nuevo necesita el historial del proyecto, no solo un placeholder local.
            $stmt = $pdo->prepare("SELECT id FROM users WHERE public_key = ?");
            $stmt->execute([$invite['inviter_identity']]);
            $inviterUserId = $stmt->fetchColumn();
            if ($inviterUserId) {
                backfill_project_sync_events($pdo, $inviterUserId, $userId, $invite['project_id']);
            }

            insert_project_member_sync_event(
                $pdo,
                $userId,
                $invite['project_id'],
                $invite['inviter_identity'],
                $invite['inviter_username'],
                'Owner',
                $deviceId
            );
            insert_project_member_sync_event(
                $pdo,
                $userId,
                $invite['project_id'],
                $identity,
                $inviteeUsername,
                $invite['role'],
                $deviceId
            );
            if ($inviterUserId) {
                insert_project_member_sync_event(
                    $pdo,
                    $inviterUserId,
                    $invite['project_id'],
                    $invite['inviter_identity'],
                    $invite['inviter_username'],
                    'Owner',
                    $deviceId
                );
                insert_project_member_sync_event(
                    $pdo,
                    $inviterUserId,
                    $invite['project_id'],
                    $identity,
                    $inviteeUsername,
                    $invite['role'],
                    $deviceId
                );
            }
            
            // Mensaje de bienvenida automático en el chronicle — para que se vea chido
            $msgId = bin2hex(random_bytes(16));
            $stmt = $pdo->prepare("INSERT INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)");
            $stmt->execute([
                $msgId,
                $invite['project_id'],
                'system',
                'System',
                "Fellowship companion joined project: " . $invite['project_name'],
                'system',
                date(DATE_RFC3339)
            ]);
            
            log_api_event($pdo, $userId, $deviceId, 'INVITATION', "Accepted invitation to project ID " . $invite['project_id']);
            echo json_encode(["status" => "success", "project_id" => $invite['project_id'], "project_name" => $invite['project_name']]);
            break;
            
        case 'decline':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $inviteId = $data['invite_id'] ?? null;
            if (!$inviteId) {
                http_response_code(400);
                echo json_encode(["error" => "Missing invite_id parameter"]);
                exit;
            }
            
            $stmt = $pdo->prepare("UPDATE project_invitations SET status = 'Declined' WHERE id = ? AND status = 'Pending'");
            $stmt->execute([$inviteId]);
            
            echo json_encode(["status" => "success"]);
            break;
            
        case 'pending':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            // Jalar invitaciones pendientes para esta identity — las que están esperando respuesta
            $stmt = $pdo->prepare("SELECT id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at FROM project_invitations WHERE invitee_identity = ? AND status = 'Pending' ORDER BY created_at DESC");
            $stmt->execute([$identity]);
            $invites = $stmt->fetchAll();
            echo json_encode($invites);
            break;

        case 'project/companions':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            // Todos los compañeros de proyectos compartidos — joined con devices para saber quién está online
            $stmt = $pdo->prepare("
                SELECT DISTINCT
                    pm.user_identity,
                    pm.user_username,
                    pm.role,
                    MAX(d.last_seen) AS last_seen
                FROM project_members pm
                INNER JOIN project_members me
                    ON me.project_id = pm.project_id AND me.user_identity = ?
                LEFT JOIN users u ON u.public_key = pm.user_identity
                LEFT JOIN devices d ON d.user_id = u.id
                WHERE pm.user_identity != ?
                GROUP BY pm.user_identity, pm.user_username, pm.role
                ORDER BY pm.user_username ASC
            ");
            $stmt->execute([$identity, $identity]);
            $companions = $stmt->fetchAll();
            echo json_encode($companions);
            break;

        case 'user/lookup':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $targetKey = $_GET['key'] ?? null;
            if (!$targetKey || strlen($targetKey) !== 64) {
                http_response_code(400);
                echo json_encode(["error" => "Missing or invalid key"]);
                break;
            }
            $username = null;
            // project_members es la fuente más confiable — fallback a chronicle, luego a invitaciones
            $stmt = $pdo->prepare("SELECT user_username FROM project_members WHERE user_identity = ? LIMIT 1");
            $stmt->execute([$targetKey]);
            $row = $stmt->fetch();
            if ($row) { $username = $row['user_username']; }
            // fall back to chronicle messages
            if (!$username) {
                $stmt = $pdo->prepare("SELECT sender_username FROM chronicle_messages WHERE sender_identity = ? LIMIT 1");
                $stmt->execute([$targetKey]);
                $row = $stmt->fetch();
                if ($row) { $username = $row['sender_username']; }
            }
            // fall back to invitations (as inviter)
            if (!$username) {
                $stmt = $pdo->prepare("SELECT inviter_username FROM project_invitations WHERE inviter_identity = ? LIMIT 1");
                $stmt->execute([$targetKey]);
                $row = $stmt->fetch();
                if ($row) { $username = $row['inviter_username']; }
            }
            if ($username) {
                echo json_encode(["username" => $username]);
            } else {
                http_response_code(404);
                echo json_encode(["error" => "User not found"]);
            }
            break;

        // ── Chronicle: mensajes, presencia y reacciones ───────────────────────────
        case 'chronicle/message':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $projId = $data['project_id'] ?? null;
            $content = $data['content'] ?? null;
            $msgType = $data['message_type'] ?? 'text';

            if (!$projId || !$content) {
                http_response_code(400);
                echo json_encode(["error" => "Missing project_id or message content"]);
                exit;
            }
            if (strlen($content) > 4000) {
                http_response_code(400);
                echo json_encode(["error" => "Message too long (max 4000 characters)"]);
                exit;
            }

            // Usar el ID del cliente para que local y servidor tengan la misma key — fallback para clientes viejos
            $msgId = (isset($data['id']) && strlen($data['id']) >= 16) ? $data['id'] : bin2hex(random_bytes(16));

            // Resolver nombre del que manda: users > project_members > devices > 'Companion'
            $senderName = null;
            $stmt = $pdo->prepare("SELECT username FROM users WHERE public_key = ? LIMIT 1");
            $stmt->execute([$identity]);
            $senderName = $stmt->fetchColumn() ?: null;
            if (!$senderName) {
                $stmt = $pdo->prepare("SELECT user_username FROM project_members WHERE user_identity = ? AND user_username != '' LIMIT 1");
                $stmt->execute([$identity]);
                $senderName = $stmt->fetchColumn() ?: null;
            }
            if (!$senderName) {
                $stmt = $pdo->prepare("SELECT device_name FROM devices WHERE id = ? LIMIT 1");
                $stmt->execute([$deviceId]);
                $senderName = $stmt->fetchColumn() ?: 'Companion';
            }

            $stmt = $pdo->prepare("INSERT IGNORE INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?)");
            $stmt->execute([
                $msgId,
                $projId,
                $identity,
                $senderName,
                $content,
                $msgType,
                date(DATE_RFC3339)
            ]);

            log_api_event($pdo, $userId, $deviceId, 'CHRONICLE', "Posted chronicle message to project $projId");
            echo json_encode(["status" => "success", "message_id" => $msgId]);
            break;
            
        case 'chronicle/messages':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $projId = $_GET['project_id'] ?? null;
            if (!$projId) {
                http_response_code(400);
                echo json_encode(["error" => "Missing project_id parameter"]);
                exit;
            }
            $since = $_GET['since'] ?? null;
            if ($since) {
                $stmt = $pdo->prepare("SELECT id, project_id, sender_identity, sender_username, content, message_type, timestamp FROM chronicle_messages WHERE project_id = ? AND timestamp > ? ORDER BY timestamp ASC");
                $stmt->execute([$projId, $since]);
            } else {
                $stmt = $pdo->prepare("SELECT id, project_id, sender_identity, sender_username, content, message_type, timestamp FROM chronicle_messages WHERE project_id = ? ORDER BY timestamp ASC");
                $stmt->execute([$projId]);
            }
            $messages = $stmt->fetchAll();
            echo json_encode($messages);
            break;

        case 'chronicle/presence':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $projId = $_GET['project_id'] ?? null;
            if (!$projId) {
                http_response_code(400);
                echo json_encode(["error" => "Missing project_id parameter"]);
                exit;
            }
            $stmt = $pdo->prepare("
                SELECT
                    pm.user_identity,
                    pm.user_username,
                    pm.role,
                    MAX(d.last_seen) AS last_seen,
                    CASE WHEN MAX(d.last_seen) > DATE_SUB(NOW(), INTERVAL 3 MINUTE) THEN 1 ELSE 0 END AS is_online
                FROM project_members pm
                LEFT JOIN users u ON u.public_key = pm.user_identity
                LEFT JOIN devices d ON d.user_id = u.id
                WHERE pm.project_id = ?
                GROUP BY pm.user_identity, pm.user_username, pm.role
                ORDER BY is_online DESC, pm.user_username ASC
            ");
            $stmt->execute([$projId]);
            $presence = $stmt->fetchAll();
            echo json_encode($presence);
            break;
            
        case 'chronicle/react':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $msgId = $data['message_id'] ?? null;
            $emoji = $data['emoji'] ?? null;
            
            if (!$msgId || !$emoji) {
                http_response_code(400);
                echo json_encode(["error" => "Missing message_id or emoji parameter"]);
                exit;
            }
            
            $stmt = $pdo->prepare("INSERT INTO message_reactions (message_id, user_identity, emoji) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE emoji = ?");
            $stmt->execute([$msgId, $identity, $emoji, $emoji]);
            
            echo json_encode(["status" => "success"]);
            break;
            
        // ── Bug reports — guarda el reporte y manda email al dev ─────────────────
        case 'report':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }

            $report = json_decode($body, true);
            if (!$report) {
                http_response_code(400);
                echo json_encode(["error" => "Invalid report payload"]);
                break;
            }

            $reportType    = substr($report['report_type'] ?? 'Unknown', 0, 30);
            $description   = $report['description'] ?? '';
            $version       = substr($report['version'] ?? '', 0, 20);
            $os            = substr($report['os'] ?? '', 0, 50);
            $arch          = substr($report['arch'] ?? '', 0, 20);
            $term          = substr($report['term'] ?? '', 0, 50);
            $termProgram   = substr($report['term_program'] ?? '', 0, 50);
            $username      = substr($report['username'] ?? '', 0, 255);
            $class         = substr($report['class'] ?? '', 0, 50);
            $level         = (int)($report['level'] ?? 0);

            $stmt = $pdo->prepare("INSERT INTO bug_reports (user_id, report_type, description, version, os, arch, term, term_program, username, class, level) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)");
            $stmt->execute([$userId, $reportType, $description, $version, $os, $arch, $term, $termProgram, $username, $class, $level]);

            // Notificación por email al dev — para enterarse sin tener que revisar la DB
            $devEmail = getenv('DEV_EMAIL') ?: 'thisdoesnotwork@gibranlp.dev';
            $subject  = "[Questline Report] {$reportType} from {$username} (v{$version})";
            $body     = "Type: {$reportType}\n";
            $body    .= "Hero: {$username} | Class: {$class} | Level: {$level}\n";
            $body    .= "Version: {$version} | OS: {$os} ({$arch})\n";
            $body    .= "Terminal: {$term} / {$termProgram}\n\n";
            $body    .= "--- Description ---\n{$description}\n";
            $headers  = "From: noreply@questline.app\r\nContent-Type: text/plain; charset=UTF-8";
            if (!mail($devEmail, $subject, $body, $headers)) {
                error_log("Questline: bug report mail() failed for report type: {$reportType}");
            }

            log_api_event($pdo, $userId, $deviceId, 'REPORT', "Report submitted: {$reportType}");
            echo json_encode(["status" => "success"]);
            break;

        // ── Backup/Recovery — la base de datos del héroe, hasta 50 MB ────────────
        case 'backup':
        case 'recovery':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            if (strlen($body) > 52428800) { // 50 MB hard cap
                http_response_code(413);
                echo json_encode(["error" => "Backup payload too large"]);
                exit;
            }
            // Sobreescribe el backup anterior — solo se guarda el más reciente, pues
            $stmt = $pdo->prepare("INSERT INTO backups (user_id, backup_data) VALUES (?, ?) ON DUPLICATE KEY UPDATE backup_data = ?, created_at = CURRENT_TIMESTAMP");
            $stmt->execute([$userId, $body, $body]);
            
            echo json_encode(["status" => "success", "message" => "Backup written successfully"]);
            break;
            
        case 'backup/latest':
        case 'recovery/latest':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $stmt = $pdo->prepare("SELECT backup_data FROM backups WHERE user_id = ?");
            $stmt->execute([$userId]);
            $backup = $stmt->fetchColumn();
            
            if (!$backup) {
                http_response_code(404);
                echo json_encode(["error" => "No backup found for this identity"]);
                exit;
            }
            
            echo $backup; // Return raw JSON dump directly
            break;
            
        // ── Global Chronicle — el feed mundial de eventos de todos los héroes ────
        case 'global_chronicle':
            if ($_SERVER['REQUEST_METHOD'] === 'GET') {
                $stmt = $pdo->prepare(
                    "SELECT id, hero_name, event_type, description, timestamp FROM global_chronicle ORDER BY timestamp DESC LIMIT 200"
                );
                $stmt->execute();
                echo json_encode($stmt->fetchAll(PDO::FETCH_ASSOC));
            } elseif ($_SERVER['REQUEST_METHOD'] === 'POST') {
                $data = json_decode($body, true);
                if (!$data || empty($data['id']) || empty($data['hero_name']) || empty($data['event_type']) || empty($data['description'])) {
                    http_response_code(400);
                    echo json_encode(["error" => "Missing required fields"]);
                    break;
                }
                // Sanitizar el nombre del héroe — no queremos scripts ni caracteres raros en el feed
                $hero = preg_replace('/[^a-zA-Z0-9 _\-]/', '', $data['hero_name']);
                $hero = substr(trim($hero), 0, 64);
                if (empty($hero)) {
                    http_response_code(400);
                    echo json_encode(["error" => "Invalid hero_name"]);
                    break;
                }
                $allowed_types = ['LevelUp','RealmComplete','Milestone','Relic','Streak','Memory','MemoryFragment','Legend','DailyAdventure','ZenTree','TreeWatering','QuestComplete','FocusSession','SidequestComplete','ReflectionWritten','ScrollCreated','StepsComplete','ChapterComplete','ClassQuest','ClassStory','WorldLore','Achievement'];
                $event_type = in_array($data['event_type'], $allowed_types) ? $data['event_type'] : 'LevelUp';
                $description = substr(strip_tags($data['description']), 0, 255);
                $ts = isset($data['timestamp']) ? substr($data['timestamp'], 0, 50) : date('c');
                $id = preg_replace('/[^a-zA-Z0-9\-]/', '', $data['id']);
                $stmt = $pdo->prepare(
                    "INSERT IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?, ?, ?, ?, ?)"
                );
                $stmt->execute([$id, $hero, $event_type, $description, $ts]);
                trigger_global_webhook($hero, $event_type, $description);
                echo json_encode(["ok" => true]);
            } else {
                http_response_code(405);
                echo json_encode(["error" => "Method not allowed"]);
            }
            break;

        // ── Living Chapters — el progreso cooperativo global del realm ────────────
        case 'chapter/active':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $chapter_id = $_GET['chapter_id'] ?? 'chapter_one';
            $chapter_id = preg_replace('/[^a-z0-9_]/', '', strtolower($chapter_id));
            if (empty($chapter_id)) { $chapter_id = 'chapter_one'; }

            // Sembrar los objetivos del Chapter One la primera vez — targets fijos del diseño
            $chapter_one_objectives = [
                'tasks_completed'     => 1000,
                'subtasks_completed'  => 2000,
                'focus_sessions'      => 500,
                'tree_waterings'      => 2000,
                'rituals_completed'   => 300,
                'reflections_written' => 750,
                'scrolls_created'     => 1000,
            ];
            if ($chapter_id === 'chapter_one') {
                $pdo->exec("INSERT IGNORE INTO chapter_progress (chapter_id, completed) VALUES ('chapter_one', 0)");
                foreach ($chapter_one_objectives as $obj_type => $target) {
                    $stmt = $pdo->prepare("INSERT IGNORE INTO chapter_objectives (chapter_id, objective_type, current_value, target_value) VALUES (?, ?, 0, ?)");
                    $stmt->execute(['chapter_one', $obj_type, $target]);
                }
            }

            $stmt = $pdo->prepare("SELECT completed, completed_at FROM chapter_progress WHERE chapter_id = ?");
            $stmt->execute([$chapter_id]);
            $cp = $stmt->fetch();

            $stmt = $pdo->prepare("SELECT objective_type, current_value, target_value FROM chapter_objectives WHERE chapter_id = ?");
            $stmt->execute([$chapter_id]);
            $objectives = [];
            while ($row = $stmt->fetch()) {
                $objectives[] = [
                    'type'    => $row['objective_type'],
                    'current' => (int)$row['current_value'],
                    'target'  => (int)$row['target_value'],
                ];
            }

            echo json_encode([
                'chapter_id'   => $chapter_id,
                'completed'    => $cp ? (bool)$cp['completed'] : false,
                'completed_at' => $cp ? $cp['completed_at'] : null,
                'objectives'   => $objectives,
            ]);
            break;

        case 'chapter/contribute':
            if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
                send_method_not_allowed();
            }
            $data = json_decode($body, true);
            $chapter_id = $data['chapter_id'] ?? 'chapter_one';
            $chapter_id = preg_replace('/[^a-z0-9_]/', '', strtolower($chapter_id));
            if (empty($chapter_id)) { $chapter_id = 'chapter_one'; }
            $contributions = $data['contributions'] ?? [];

            if (!is_array($contributions) || empty($contributions)) {
                http_response_code(400);
                echo json_encode(["error" => "Missing or empty contributions"]);
                break;
            }

            $allowed_types = ['tasks_completed', 'subtasks_completed', 'focus_sessions', 'tree_waterings', 'rituals_completed', 'reflections_written', 'scrolls_created'];

            $pdo->beginTransaction();
            $newly_completed = false;
            $remaining = 1;
            try {
                foreach ($contributions as $obj_type => $amount) {
                    if (!in_array($obj_type, $allowed_types)) continue;
                    $amount = max(0, min(10000, (int)$amount));
                    if ($amount === 0) continue;

                    // Sumar la contribución al contador global del objetivo
                    $stmt = $pdo->prepare("UPDATE chapter_objectives SET current_value = current_value + ? WHERE chapter_id = ? AND objective_type = ?");
                    $stmt->execute([$amount, $chapter_id, $obj_type]);

                    // Track per-user contribution
                    $stmt = $pdo->prepare("
                        INSERT INTO chapter_contributions (user_id, chapter_id, objective_type, total_contributed)
                        VALUES (?, ?, ?, ?)
                        ON DUPLICATE KEY UPDATE total_contributed = total_contributed + VALUES(total_contributed)
                    ");
                    $stmt->execute([$userId, $chapter_id, $obj_type, $amount]);
                }

                // Verificar si ya se completaron todos los objetivos del capítulo
                $stmt = $pdo->prepare("
                    SELECT COUNT(*) FROM chapter_objectives
                    WHERE chapter_id = ? AND current_value < target_value
                ");
                $stmt->execute([$chapter_id]);
                $remaining = (int)$stmt->fetchColumn();

                // UPDATE atómico para evitar race conditions — la subquery re-verifica adentro del UPDATE
                if ($remaining === 0) {
                    $stmt = $pdo->prepare("
                        UPDATE chapter_progress
                        SET completed = 1, completed_at = NOW()
                        WHERE chapter_id = ?
                          AND completed = 0
                          AND (SELECT COUNT(*) FROM chapter_objectives WHERE chapter_id = ? AND current_value < target_value) = 0
                    ");
                    $stmt->execute([$chapter_id, $chapter_id]);
                    $newly_completed = $stmt->rowCount() > 0;

                    if ($newly_completed) {
                        // El capítulo se completó — publicar en el global chronicle para que todos lo vean
                        $completion_id = bin2hex(random_bytes(16));
                        $stmt = $pdo->prepare("INSERT IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?, 'The Realm', 'ChapterComplete', ?, ?)");
                        $stmt->execute([
                            $completion_id,
                            'The Notification Swarm has been dispersed. Heroes across the Realm completed the Chapter.',
                            date('c'),
                        ]);
                    }
                }

                $pdo->commit();
            } catch (PDOException $e) {
                // Sólo hay transacción activa antes del commit — esto es seguro
                if ($pdo->inTransaction()) { $pdo->rollBack(); }
                throw $e;
            }

            // Generar entradas del activity feed FUERA de la transacción — best effort, no rollback
            try {
                // Resolver nombre del héroe: users.username → project_members → devices
                $heroName = null;
                $stmt = $pdo->prepare("SELECT username FROM users WHERE id = ? AND username IS NOT NULL AND username != ''");
                $stmt->execute([$userId]);
                $heroName = $stmt->fetchColumn() ?: null;
                if (!$heroName) {
                    $stmt = $pdo->prepare("SELECT user_username FROM project_members WHERE user_identity = ? AND user_username != '' LIMIT 1");
                    $stmt->execute([$identity]);
                    $heroName = $stmt->fetchColumn() ?: null;
                }
                if (!$heroName) {
                    $stmt = $pdo->prepare("SELECT device_name FROM devices WHERE user_id = ? LIMIT 1");
                    $stmt->execute([$userId]);
                    $heroName = $stmt->fetchColumn() ?: null;
                }

                if ($heroName) {
                    $activityTypes = [
                        'tasks_completed'     => ['QuestComplete',      'quest',           'quests'],
                        'subtasks_completed'  => ['StepsComplete',      'step',            'steps'],
                        'focus_sessions'      => ['FocusSession',       'focus session',   'focus sessions'],
                        'rituals_completed'   => ['SidequestComplete',  'sidequest',       'sidequests'],
                        'reflections_written' => ['ReflectionWritten',  'reflection',      'reflections'],
                        'tree_waterings'      => ['ZenTree',            'Zen Tree watering', 'Zen Tree waterings'],
                        'scrolls_created'     => ['ScrollCreated',      'scroll',          'scrolls'],
                    ];
                    foreach ($contributions as $obj_type => $amount) {
                        $amount = (int)$amount;
                        if ($amount <= 0 || !isset($activityTypes[$obj_type])) continue;
                        [$eventType, $singular, $plural] = $activityTypes[$obj_type];
                        $noun = $amount === 1 ? $singular : $plural;
                        $desc = "completed {$amount} {$noun}.";
                        if ($obj_type === 'tree_waterings') {
                            $desc = $amount === 1 ? "watered The Evergrowth." : "watered The Evergrowth {$amount} times.";
                        } elseif ($obj_type === 'focus_sessions') {
                            $desc = $amount === 1 ? "honored a focus session." : "honored {$amount} focus sessions.";
                        } elseif ($obj_type === 'reflections_written') {
                            $desc = $amount === 1 ? "wrote a reflection." : "wrote {$amount} reflections.";
                        }
                        $entryId = bin2hex(random_bytes(16));
                        $ins = $pdo->prepare("INSERT IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?, ?, ?, ?, ?)");
                        $ins->execute([$entryId, $heroName, $eventType, $desc, date('c')]);
                    }
                }
            } catch (PDOException $e) {
                // El activity feed falla silen... no matamos el request por esto
                error_log("[Questline API] chapter/contribute activity feed error: " . $e->getMessage());
            }

            echo json_encode(["ok" => true, "completed" => $remaining === 0, "newly_completed" => $newly_completed]);
            break;

        case 'chapter/history':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            // Capítulos completados con la aportación personal del héroe — para el historial
            $stmt = $pdo->prepare("
                SELECT
                    cp.chapter_id,
                    cp.completed_at,
                    COALESCE(SUM(cc.total_contributed), 0) AS personal_contribution
                FROM chapter_progress cp
                LEFT JOIN chapter_contributions cc
                    ON cc.chapter_id = cp.chapter_id AND cc.user_id = ?
                WHERE cp.completed = 1
                GROUP BY cp.chapter_id, cp.completed_at
                ORDER BY cp.completed_at DESC
            ");
            $stmt->execute([$userId]);
            $history = [];
            while ($row = $stmt->fetch()) {
                $history[] = [
                    'chapter_id'           => $row['chapter_id'],
                    'title'                => 'Chapter One: The Notification Swarm',
                    'completed_at'         => $row['completed_at'],
                    'personal_contribution' => (int)$row['personal_contribution'],
                ];
            }
            echo json_encode($history);
            break;

        case 'chapter/my-contributions':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $chapter_id = $_GET['chapter_id'] ?? 'chapter_one';
            $chapter_id = preg_replace('/[^a-z0-9_]/', '', strtolower($chapter_id));
            if (empty($chapter_id)) { $chapter_id = 'chapter_one'; }

            $stmt = $pdo->prepare("
                SELECT objective_type, total_contributed
                FROM chapter_contributions
                WHERE user_id = ? AND chapter_id = ?
            ");
            $stmt->execute([$userId, $chapter_id]);
            $totals = [];
            while ($row = $stmt->fetch()) {
                $totals[$row['objective_type']] = (int)$row['total_contributed'];
            }
            echo json_encode(['chapter_id' => $chapter_id, 'totals' => $totals]);
            break;

        // ── Webapp supporter status — verifica que el usuario siga siendo supporter ──
        case 'webapp/supporter-status':
            if ($_SERVER['REQUEST_METHOD'] !== 'GET') {
                send_method_not_allowed();
            }
            $stmt = $pdo->prepare("SELECT supporter FROM users WHERE id = ?");
            $stmt->execute([$userId]);
            $row = $stmt->fetch();
            echo json_encode(["supporter" => $row ? (bool)$row['supporter'] : false]);
            break;

        // ── Webapp snapshot — latest singleton entities (user, zen_tree) for initial load ──
        case 'webapp/snapshot':
            $snapshot = [];
            foreach (['user', 'zen_tree'] as $entityType) {
                $stmt = $pdo->prepare("SELECT payload FROM sync_events WHERE user_id = ? AND entity_type = ? ORDER BY seq DESC LIMIT 1");
                $stmt->execute([$userId, $entityType]);
                $payload = $stmt->fetchColumn();
                if ($payload !== false && $payload !== '') {
                    $decoded = json_decode($payload, true);
                    if ($decoded) $snapshot[$entityType] = $decoded;
                }
            }
            echo json_encode($snapshot);
            break;

        default:
            http_response_code(404);
            echo json_encode(["error" => "Not found"]);
            break;
    }
} catch (PDOException $e) {
    log_api_event($pdo, $userId, $deviceId, 'API_ERROR', $e->getMessage());
    error_log("[Questline API] Unhandled DB error: " . $e->getMessage());
    http_response_code(500);
    echo json_encode(["error" => "Service temporarily unavailable"]);
}

// ── Helper: verificar firmas Ed25519 con libsodium — si no está cargado, falla nomás ──
function verify_ed25519_signature($publicKeyHex, $signatureHex, $message) {
    try {
        if (!extension_loaded('sodium')) {
            // Sin libsodium no hay crypto — en prod esto no debería pasar nunca
            return false;
        }
        $pub = sodium_hex2bin($publicKeyHex);
        $sig = sodium_hex2bin($signatureHex);
        if (strlen($pub) !== 32 || strlen($sig) !== 64) {
            return false;
        }
        return sodium_crypto_sign_verify_detached($sig, $message, $pub);
    } catch (Exception $e) {
        return false;
    }
}

function is_project_scoped_sync_type($entityType) {
    return in_array($entityType, [
        'task',
        'note',
        'journal_entry',
        'milestone',
        'focus_session',
        'codex',
        'task_assignment',
        'project_member',
        'chronicle_message'
    ], true);
}

function extract_project_id_from_payload($payload) {
    if (!$payload || !is_string($payload)) {
        return null;
    }
    $payloadObj = json_decode($payload, true);
    if (!is_array($payloadObj) || empty($payloadObj['project_id'])) {
        return null;
    }
    return $payloadObj['project_id'];
}

function backfill_project_sync_events($pdo, $sourceUserId, $targetUserId, $projectId) {
    if (!$sourceUserId || !$targetUserId || !$projectId || $sourceUserId === $targetUserId) {
        return 0;
    }

    $select = $pdo->prepare("
        SELECT id, entity_type, entity_id, operation, payload, created_at, device_id
        FROM sync_events
        WHERE user_id = ?
        ORDER BY seq ASC
    ");
    $insert = $pdo->prepare("
        INSERT IGNORE INTO sync_events
            (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    ");

    $copied = 0;
    $select->execute([$sourceUserId]);
    while ($event = $select->fetch(PDO::FETCH_ASSOC)) {
        $eventProjectId = null;
        if ($event['entity_type'] === 'project') {
            $eventProjectId = $event['entity_id'];
        } elseif (is_project_scoped_sync_type($event['entity_type'])) {
            $eventProjectId = extract_project_id_from_payload($event['payload'] ?? '');
        }

        if ($eventProjectId !== $projectId) {
            continue;
        }

        $backfillId = md5($event['id'] . $targetUserId . 'project_backfill');
        $insert->execute([
            $backfillId,
            $targetUserId,
            $event['entity_type'],
            $event['entity_id'],
            $event['operation'],
            $event['payload'] ?? '',
            $event['created_at'],
            $event['device_id'] ?? ''
        ]);
        $copied += $insert->rowCount();
    }

    return $copied;
}

function insert_project_member_sync_event($pdo, $targetUserId, $projectId, $memberIdentity, $memberUsername, $role, $deviceId) {
    if (!$targetUserId || !$projectId || !$memberIdentity) {
        return;
    }

    $payload = json_encode([
        'project_id' => $projectId,
        'user_identity' => $memberIdentity,
        'user_username' => $memberUsername,
        'role' => $role
    ]);
    $eventId = md5($targetUserId . $projectId . $memberIdentity . $role . 'project_member_accept');
    $stmt = $pdo->prepare("
        INSERT IGNORE INTO sync_events
            (id, user_id, entity_type, entity_id, operation, payload, created_at, device_id)
        VALUES (?, ?, 'project_member', ?, 'upsert', ?, ?, ?)
    ");
    $stmt->execute([
        $eventId,
        $targetUserId,
        $projectId . '__' . $memberIdentity,
        $payload,
        date(DATE_RFC3339),
        $deviceId ?? ''
    ]);
}

// Logger de eventos — errores, auth failures, acciones importantes
function log_api_event($pdo, $userId, $deviceId, $type, $msg) {
    try {
        $stmt = $pdo->prepare("INSERT INTO api_logs (user_id, device_id, log_type, message) VALUES (?, ?, ?, ?)");
        $stmt->execute([$userId, $deviceId, $type, $msg]);
    } catch (Exception $e) {
        // Suppress logger errors
    }
}

function send_method_not_allowed() {
    http_response_code(405);
    echo json_encode(["error" => "Method not allowed"]);
    exit;
}

// ── Autocreación del schema — corre solo la primera vez que se instala el server ──
function setup_tables($pdo) {
    $sql = "
    CREATE TABLE IF NOT EXISTS users (
        id VARCHAR(36) PRIMARY KEY,
        public_key VARCHAR(64) UNIQUE NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS devices (
        id VARCHAR(36) PRIMARY KEY,
        user_id VARCHAR(36) NOT NULL,
        device_name VARCHAR(255) NOT NULL,
        last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS sync_events (
        id VARCHAR(36) PRIMARY KEY,
        user_id VARCHAR(36) NOT NULL,
        entity_type VARCHAR(50) NOT NULL,
        entity_id VARCHAR(36) NOT NULL,
        operation VARCHAR(20) NOT NULL,
        payload LONGTEXT NOT NULL,
        created_at VARCHAR(50) NOT NULL,
        INDEX idx_sync_events_user_entity (user_id, entity_type, entity_id)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS project_members (
        project_id VARCHAR(36) NOT NULL,
        user_identity VARCHAR(64) NOT NULL,
        user_username VARCHAR(255) NOT NULL,
        role VARCHAR(50) NOT NULL,
        PRIMARY KEY (project_id, user_identity)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS project_invitations (
        id VARCHAR(36) PRIMARY KEY,
        project_id VARCHAR(36) NOT NULL,
        project_name VARCHAR(255) NOT NULL,
        inviter_identity VARCHAR(64) NOT NULL,
        inviter_username VARCHAR(255) NOT NULL,
        invitee_identity VARCHAR(64) NOT NULL,
        role VARCHAR(50) NOT NULL,
        status VARCHAR(20) NOT NULL DEFAULT 'Pending',
        created_at VARCHAR(50) NOT NULL,
        INDEX idx_invitee (invitee_identity)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS chronicle_messages (
        id VARCHAR(36) PRIMARY KEY,
        project_id VARCHAR(36) NOT NULL,
        sender_identity VARCHAR(64) NOT NULL,
        sender_username VARCHAR(255) NOT NULL,
        content TEXT NOT NULL,
        message_type VARCHAR(50) NOT NULL,
        timestamp VARCHAR(50) NOT NULL,
        INDEX idx_project_msg (project_id)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS message_reactions (
        message_id VARCHAR(36) NOT NULL,
        user_identity VARCHAR(64) NOT NULL,
        emoji VARCHAR(50) NOT NULL,
        PRIMARY KEY (message_id, user_identity, emoji),
        FOREIGN KEY (message_id) REFERENCES chronicle_messages(id) ON DELETE CASCADE
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS project_permissions (
        role VARCHAR(50) NOT NULL,
        permission_key VARCHAR(50) NOT NULL,
        allowed TINYINT(1) NOT NULL DEFAULT 0,
        PRIMARY KEY (role, permission_key)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS backups (
        user_id VARCHAR(36) PRIMARY KEY,
        backup_data LONGTEXT NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
        FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS nonces (
        user_id VARCHAR(36) NOT NULL,
        nonce VARCHAR(128) NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        PRIMARY KEY (user_id, nonce)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS api_logs (
        id INT AUTO_INCREMENT PRIMARY KEY,
        user_id VARCHAR(36) NULL,
        device_id VARCHAR(36) NULL,
        log_type VARCHAR(50) NOT NULL,
        message TEXT NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS bug_reports (
        id INT AUTO_INCREMENT PRIMARY KEY,
        user_id VARCHAR(36) NULL,
        report_type VARCHAR(30) NOT NULL,
        description TEXT NOT NULL,
        version VARCHAR(20) NULL,
        os VARCHAR(50) NULL,
        arch VARCHAR(20) NULL,
        term VARCHAR(50) NULL,
        term_program VARCHAR(50) NULL,
        username VARCHAR(255) NULL,
        class VARCHAR(50) NULL,
        level INT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS chapter_progress (
        chapter_id VARCHAR(50) PRIMARY KEY,
        completed TINYINT(1) NOT NULL DEFAULT 0,
        completed_at TIMESTAMP NULL,
        last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS chapter_objectives (
        chapter_id VARCHAR(50) NOT NULL,
        objective_type VARCHAR(50) NOT NULL,
        current_value BIGINT UNSIGNED NOT NULL DEFAULT 0,
        target_value BIGINT UNSIGNED NOT NULL,
        PRIMARY KEY (chapter_id, objective_type)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS chapter_contributions (
        id INT AUTO_INCREMENT PRIMARY KEY,
        user_id VARCHAR(36) NOT NULL,
        chapter_id VARCHAR(50) NOT NULL,
        objective_type VARCHAR(50) NOT NULL,
        total_contributed BIGINT UNSIGNED NOT NULL DEFAULT 0,
        UNIQUE KEY uq_chapter_contrib (user_id, chapter_id, objective_type)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    CREATE TABLE IF NOT EXISTS global_chronicle (
        id VARCHAR(64) PRIMARY KEY,
        hero_name VARCHAR(100) NOT NULL DEFAULT '',
        event_type VARCHAR(50) NOT NULL DEFAULT '',
        description TEXT NOT NULL DEFAULT '',
        timestamp VARCHAR(50) NOT NULL DEFAULT '',
        INDEX idx_gc_timestamp (timestamp)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

    INSERT IGNORE INTO project_permissions (role, permission_key, allowed) VALUES
    ('Owner', 'write', 1),
    ('Owner', 'invite', 1),
    ('Owner', 'chat', 1),
    ('Steward', 'write', 1),
    ('Steward', 'invite', 1),
    ('Steward', 'chat', 1),
    ('Companion', 'write', 1),
    ('Companion', 'invite', 0),
    ('Companion', 'chat', 1),
    ('Observer', 'write', 0),
    ('Observer', 'invite', 0),
    ('Observer', 'chat', 1);
    ";
    $pdo->exec($sql);
}

function trigger_webhooks($pdo, $userId, $eventType, $entityId, $operation, $payload, $eventId = '', $timestamp = '') {
    try {
        $stmt = $pdo->prepare("SELECT url, events, secret FROM webhooks WHERE user_id = ?");
        $stmt->execute([$userId]);
        $webhooks = $stmt->fetchAll();
        if (empty($webhooks)) return;

        // Full sync-event body for non-Discord webhooks (e.g. webapp mirror)
        $syncBody = json_encode([
            "event_id"    => $eventId ?: bin2hex(random_bytes(8)),
            "entity_type" => $eventType,
            "entity_id"   => $entityId,
            "operation"   => $operation,
            "content"     => $payload,
            "user_id"     => $userId,
            "timestamp"   => $timestamp ?: date(DATE_RFC3339),
        ]);

        // Legacy body for Discord and other integrations
        $body = json_encode([
            "event"     => $eventType,
            "operation" => $operation,
            "user_id"   => $userId,
            "timestamp" => $timestamp ?: date(DATE_RFC3339),
            "payload"   => json_decode($payload, true) ?: $payload
        ]);

        foreach ($webhooks as $wh) {
            $allowedEvents = explode(',', $wh['events']);
            if (!in_array('*', $allowedEvents) && !in_array($eventType, $allowedEvents)) {
                // For non-Discord non-wildcard hooks, also skip if not in the legacy list
                if (!str_contains($wh['url'], 'discord.com/api/webhooks/')) {
                    // Non-Discord hooks with specific events still filter; pass-through with '*'
                }
                continue;
            }

            $isDiscord = str_contains($wh['url'], 'discord.com/api/webhooks/');
            // Non-Discord hooks receive the full sync-event body; Discord keeps the legacy format
            $postBody = $isDiscord ? $body : $syncBody;

            if ($isDiscord) {
                $nameStmt = $pdo->prepare("SELECT username, class, level FROM users WHERE id = ?");
                $nameStmt->execute([$userId]);
                $userObj = $nameStmt->fetch() ?: ["username" => "Traveler", "class" => "Neutral", "level" => 1];

                $user_name = $userObj['username'] ?? 'Traveler';
                $event_msg = "";

                $contentObj = json_decode($payload, true);
                if (!is_array($contentObj)) {
                    $contentObj = $payload;
                }

                if ($eventType === 'milestone') {
                    $mName = is_array($contentObj) ? ($contentObj['name'] ?? 'Milestone') : 'Milestone';
                    $event_msg = "{$user_name} has completed the {$mName} milestone.";
                } elseif ($eventType === 'streak') {
                    $sCount = is_array($contentObj) ? ($contentObj['streak_count'] ?? '0') : $contentObj;
                    $event_msg = "{$user_name} has maintained a streak of {$sCount} days.";
                } elseif ($eventType === 'zen_tree') {
                    $event_msg = "{$user_name} has nurtured The Evergrowth.";
                } elseif ($eventType === 'task') {
                    $tName = is_array($contentObj) ? ($contentObj['title'] ?? 'Task') : 'Task';
                    $event_msg = "{$user_name} has completed: {$tName}.";
                } elseif ($eventType === 'user_stats') {
                    $lvl = is_array($contentObj) ? ($contentObj['level'] ?? $userObj['level']) : $userObj['level'];
                    $event_msg = "{$user_name} has reached Level {$lvl}.";
                } else {
                    $event_msg = "{$user_name} has updated the Chronicle ({$eventType}).";
                }

                $ironic_quotes = [
                    "The Realm grows stronger. Or at least, slightly less disorganized.",
                    "A monumental achievement that will be forgotten by tomorrow.",
                    "The Chronicle is impressed. The Evergrowth remains completely indifferent.",
                    "The Notification Swarm retreated, if only out of sheer embarrassment.",
                    "Proof that sufficient database entries can simulate actual productivity.",
                    "Somewhere, a manager is wondering why you aren't doing actual work instead of leveling up.",
                    "The Realm survives. Unfortunately, so does the backlog.",
                    "A triumph of discipline over common sense.",
                    "A significant milestone on the journey to doing exactly what you were supposed to do three days ago.",
                    "A heroic effort, assuming the standard for heroism has dropped considerably."
                ];
                $quote = $ironic_quotes[array_rand($ironic_quotes)];

                $messageText = "━━━━━━━━━━━━━━━━━━━━━━\nTHE CHRONICLER RECORDS\n━━━━━━━━━━━━━━━━━━━━━━\n\n{$event_msg}\n\n{$quote}";

                $discordPayload = [
                    "username" => "Questline Chronicle",
                    "content" => $messageText
                ];
                $postBody = json_encode($discordPayload);
            }

            $ch = curl_init($wh['url']);
            curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
            curl_setopt($ch, CURLOPT_POST, true);
            curl_setopt($ch, CURLOPT_POSTFIELDS, $postBody);
            curl_setopt($ch, CURLOPT_TIMEOUT, 3);
            curl_setopt($ch, CURLOPT_CONNECTTIMEOUT, 2);

            $curlHeaders = [
                'Content-Type: application/json',
                'User-Agent: Questline-Webhook/1.0'
            ];
            if (!empty($wh['secret'])) {
                $curlHeaders[] = 'X-Questline-Signature: ' . hash_hmac('sha256', $postBody, $wh['secret']);
            }
            curl_setopt($ch, CURLOPT_HTTPHEADER, $curlHeaders);

            curl_exec($ch);
            curl_close($ch);
        }
    } catch (Exception $e) {
        // Suppress errors to keep sync transaction atomic and fast
    }
}

function trigger_global_webhook($hero, $eventType, $description) {
    try {
        $webhookUrl = getenv('DISCORD_WEBHOOK_URL');
        if (empty($webhookUrl)) return;

        $event_msg = "";
        if ($eventType === 'LevelUp') {
            $event_msg = "{$hero} has reached Level " . preg_replace('/[^0-9]/', '', $description) . ".";
        } elseif ($eventType === 'Streak') {
            $event_msg = "{$hero} has maintained a streak of " . preg_replace('/[^0-9]/', '', $description) . " days.";
        } elseif ($eventType === 'TreeWatering' || $eventType === 'ZenTree') {
            $event_msg = "{$hero} has nurtured The Evergrowth.";
        } elseif ($eventType === 'Milestone') {
            $event_msg = "{$hero} has completed a milestone: {$description}.";
        } elseif ($eventType === 'QuestComplete' || $eventType === 'task') {
            $event_msg = "{$hero} has completed: {$description}.";
        } else {
            $event_msg = "{$hero}: {$description}";
        }

        $ironic_quotes = [
            "The Realm grows stronger. Or at least, slightly less disorganized.",
            "A monumental achievement that will be forgotten by tomorrow.",
            "The Chronicle is impressed. The Evergrowth remains completely indifferent.",
            "The Notification Swarm retreated, if only out of sheer embarrassment.",
            "Proof that sufficient database entries can simulate actual productivity.",
            "Somewhere, a manager is wondering why you aren't doing actual work instead of leveling up.",
            "The Realm survives. Unfortunately, so does the backlog.",
            "A triumph of discipline over common sense.",
            "A significant milestone on the journey to doing exactly what you were supposed to do three days ago.",
            "A heroic effort, assuming the standard for heroism has dropped considerably."
        ];
        $quote = $ironic_quotes[array_rand($ironic_quotes)];

        $messageText = "━━━━━━━━━━━━━━━━━━━━━━\nTHE CHRONICLER RECORDS\n━━━━━━━━━━━━━━━━━━━━━━\n\n{$event_msg}\n\n{$quote}";

        $discordPayload = [
            "username" => "Questline Chronicle",
            "content" => $messageText
        ];

        $ch = curl_init($webhookUrl);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($ch, CURLOPT_POST, true);
        curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($discordPayload));
        curl_setopt($ch, CURLOPT_TIMEOUT, 3);
        curl_setopt($ch, CURLOPT_CONNECTTIMEOUT, 2);
        curl_setopt($ch, CURLOPT_HTTPHEADER, ['Content-Type: application/json']);
        curl_exec($ch);
        curl_close($ch);
    } catch (Exception $e) {
        // Suppress errors
    }
}
?>
