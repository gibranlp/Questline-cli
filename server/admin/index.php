<?php
// ─────────────────────────────────────────────────────────────────────────────
// admin/index.php — el panel de administración del servidor, solo para los admins
// ─────────────────────────────────────────────────────────────────────────────

// Security headers — doble protección además del .htaccess, por si las dudas
header("X-Content-Type-Options: nosniff");
header("X-Frame-Options: DENY");
header("Referrer-Policy: no-referrer");
header("Cache-Control: no-store, no-cache, must-revalidate, private");
header("Pragma: no-cache");
header("Expires: Thu, 01 Jan 1970 00:00:00 GMT");

session_start();

require_once dirname(__DIR__) . '/load_env.php';

// Conexión a la DB — credenciales del .env, no hardcodeadas
$db_host = getenv('DB_HOST') ?: 'localhost';
$db_name = getenv('DB_NAME') ?: 'questline';
$db_user = getenv('DB_USER') ?: 'root';
$db_pass = getenv('DB_PASS') ?: '';

try {
    $pdo = new PDO("mysql:host=$db_host;dbname=$db_name;charset=utf8mb4", $db_user, $db_pass, [
        PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION,
        PDO::ATTR_DEFAULT_FETCH_MODE => PDO::FETCH_ASSOC
    ]);
} catch (PDOException $e) {
    error_log("[Questline Admin] DB connection failed: " . $e->getMessage());
    http_response_code(500);
    die("Service temporarily unavailable.");
}

// ── Auth HTTP Basic — si no pasan usuario/passcode del .env, ni entran ────────
$admin_username = getenv('ADMIN_USER') ?: 'admin';
$admin_passcode = getenv('ADMIN_PASSCODE') ?: '';

if (isset($_GET['logout'])) {
    session_destroy();
    header('HTTP/1.1 401 Unauthorized');
    exit('Logged out successfully.');
}

$auth_ok = isset($_SERVER['PHP_AUTH_USER'])
    && hash_equals($admin_username, $_SERVER['PHP_AUTH_USER'])
    && hash_equals($admin_passcode, $_SERVER['PHP_AUTH_PW'] ?? '');

if (!$auth_ok) {
    // Sleep de 1 segundo en cada intento fallido — no es Redis pero frena los brute-force caseros
    sleep(1);
    header('WWW-Authenticate: Basic realm="Admin"');
    header('HTTP/1.1 401 Unauthorized');
    exit('Unauthorized.');
}

// ── Token CSRF — cualquier form que cambie datos necesita incluir este token ───
if (empty($_SESSION['csrf_token'])) {
    $_SESSION['csrf_token'] = bin2hex(random_bytes(32));
}
$csrf_token = $_SESSION['csrf_token'];


// ── Acciones POST del panel — borrar usuarios, limpiar logs, reset total ──────
if ($_SERVER['REQUEST_METHOD'] === 'POST' && isset($_POST['action'])) {
    // CSRF validation — all state-changing forms must include the token
    if (!isset($_POST['csrf_token']) || !hash_equals($csrf_token, $_POST['csrf_token'])) {
        http_response_code(403);
        die("Security Error: Invalid or missing CSRF token. Go back and try again.");
    }
    $action = $_POST['action'];
    $success_msg = "";
    
    try {
        if ($action === 'delete_user') {
            $user_id = $_POST['user_id'] ?? '';
            if (!empty($user_id)) {
                $stmt = $pdo->prepare("DELETE FROM users WHERE id = ?");
                $stmt->execute([$user_id]);
                // Limpiar datos que no tienen CASCADE — hay que borrar a mano, qué rollo
                $stmt = $pdo->prepare("DELETE FROM nonces WHERE user_id = ?");
                $stmt->execute([$user_id]);
                $stmt = $pdo->prepare("DELETE FROM sync_events WHERE user_id = ?");
                $stmt->execute([$user_id]);
                $success_msg = "User successfully deleted.";
            }
        } elseif ($action === 'delete_backup') {
            $user_id = $_POST['user_id'] ?? '';
            if (!empty($user_id)) {
                $stmt = $pdo->prepare("DELETE FROM backups WHERE user_id = ?");
                $stmt->execute([$user_id]);
                $success_msg = "Backup successfully deleted.";
            }
        } elseif ($action === 'clear_logs') {
            $pdo->exec("TRUNCATE TABLE api_logs");
            $success_msg = "API logs cleared.";
        } elseif ($action === 'reset_all') {
            // PELIGRO: borra absolutamente todo — FK checks off para truncar sin orden
            $pdo->exec("SET FOREIGN_KEY_CHECKS = 0;");
            $pdo->exec("TRUNCATE TABLE users;");
            $pdo->exec("TRUNCATE TABLE devices;");
            $pdo->exec("TRUNCATE TABLE sync_events;");
            $pdo->exec("TRUNCATE TABLE chronicle_messages;");
            $pdo->exec("TRUNCATE TABLE message_reactions;");
            $pdo->exec("TRUNCATE TABLE project_members;");
            $pdo->exec("TRUNCATE TABLE project_invitations;");
            $pdo->exec("TRUNCATE TABLE project_permissions;");
            $pdo->exec("TRUNCATE TABLE backups;");
            $pdo->exec("TRUNCATE TABLE nonces;");
            $pdo->exec("TRUNCATE TABLE api_logs;");
            $pdo->exec("SET FOREIGN_KEY_CHECKS = 1;");
            
            // Reinsertar los permisos default — sin esto los roles no jalan
            $pdo->exec("INSERT IGNORE INTO project_permissions (role, permission_key, allowed) VALUES
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
            ('Observer', 'chat', 1);");
            
            $success_msg = "All database tables reset to default state.";
        }
        
        header("Location: " . $_SERVER['PHP_SELF'] . "?msg=" . urlencode($success_msg));
        exit;
    } catch (PDOException $e) {
        header("Location: " . $_SERVER['PHP_SELF'] . "?err=" . urlencode("Action failed: " . $e->getMessage()));
        exit;
    }
}

// ── Stats y datos para mostrar en el panel — todo read-only desde aquí ─────────
$users_count = $pdo->query("SELECT COUNT(*) FROM users")->fetchColumn();
$devices_count = $pdo->query("SELECT COUNT(*) FROM devices")->fetchColumn();
$sync_queue_count = $pdo->query("SELECT COUNT(*) FROM sync_events")->fetchColumn();
$messages_count = $pdo->query("SELECT COUNT(*) FROM chronicle_messages")->fetchColumn();
$errors_count = $pdo->query("SELECT COUNT(*) FROM api_logs WHERE log_type = 'API_ERROR' OR log_type = 'AUTH_FAILURE'")->fetchColumn();
$backups_count = $pdo->query("SELECT COUNT(*) FROM backups")->fetchColumn();

// Usuarios con nombre resuelto — project_members primero, luego chronicle si no aparece
$users = $pdo->query("
    SELECT
        u.id,
        u.public_key,
        u.created_at,
        COALESCE(
            (SELECT pm.user_username FROM project_members pm
             WHERE pm.user_identity = u.public_key LIMIT 1),
            (SELECT cm.sender_username FROM chronicle_messages cm
             WHERE cm.sender_identity = u.public_key
             ORDER BY cm.timestamp DESC LIMIT 1),
            '—'
        ) AS username
    FROM users u
    ORDER BY u.created_at DESC
    LIMIT 50
")->fetchAll();

// Dispositivos, mensajes recientes, sync queue, backups y logs de error
$devices = $pdo->query("SELECT d.id, d.device_name, d.last_seen, u.public_key FROM devices d JOIN users u ON d.user_id = u.id ORDER BY d.last_seen DESC LIMIT 20")->fetchAll();

// Mensajes y sync events recientes — para monitorear actividad
$messages = $pdo->query("SELECT project_id, sender_username, content, timestamp FROM chronicle_messages ORDER BY timestamp DESC LIMIT 15")->fetchAll();

// Eventos de sync recientes
$sync_events = $pdo->query("SELECT entity_type, entity_id, operation, created_at FROM sync_events ORDER BY created_at DESC LIMIT 15")->fetchAll();

// Backups y error logs
$backups = $pdo->query("SELECT b.user_id, b.created_at, LENGTH(b.backup_data) as size, u.public_key FROM backups b JOIN users u ON b.user_id = u.id ORDER BY b.created_at DESC LIMIT 20")->fetchAll();

// Logs de errores y auth failures — los primeros 20 más recientes
$error_logs = $pdo->query("SELECT log_type, message, created_at FROM api_logs ORDER BY created_at DESC LIMIT 20")->fetchAll();

?>
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Questline Cloud Chronicle Admin Dashboard</title>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;800&family=JetBrains+Mono:wght@400;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg-color: #0d0e12;
            --panel-bg: #14161f;
            --accent-primary: #8b5cf6;
            --accent-secondary: #06b6d4;
            --text-color: #e2e8f0;
            --text-muted: #64748b;
            --border-color: #272a37;
            --success: #10b981;
            --warning: #f59e0b;
            --danger: #ef4444;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        body {
            background-color: var(--bg-color);
            color: var(--text-color);
            font-family: 'Outfit', sans-serif;
            line-height: 1.6;
            padding: 2rem;
        }

        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
            border-bottom: 2px solid var(--border-color);
            padding-bottom: 1rem;
        }

        header h1 {
            font-size: 2rem;
            font-weight: 800;
            background: linear-gradient(135deg, var(--accent-primary), var(--accent-secondary));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        header a {
            background-color: var(--panel-bg);
            color: var(--text-color);
            border: 1px solid var(--border-color);
            padding: 0.5rem 1rem;
            text-decoration: none;
            border-radius: 6px;
            font-weight: 600;
            transition: all 0.2s ease;
        }

        header a:hover {
            border-color: var(--accent-primary);
            color: var(--accent-primary);
        }

        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2.5rem;
        }

        .stat-card {
            background-color: var(--panel-bg);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 1.5rem;
            text-align: center;
            position: relative;
            overflow: hidden;
            transition: transform 0.2s ease;
        }

        .stat-card:hover {
            transform: translateY(-4px);
        }

        .stat-card::before {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            width: 4px;
            height: 100%;
            background-color: var(--accent-primary);
        }

        .stat-card.cyan::before {
            background-color: var(--accent-secondary);
        }

        .stat-card.danger::before {
            background-color: var(--danger);
        }

        .stat-card.success::before {
            background-color: var(--success);
        }

        .stat-value {
            font-size: 2.5rem;
            font-weight: 800;
            font-family: 'JetBrains Mono', monospace;
            margin: 0.5rem 0;
        }

        .stat-label {
            color: var(--text-muted);
            text-transform: uppercase;
            font-size: 0.8rem;
            letter-spacing: 1px;
            font-weight: 600;
        }

        .dashboard-grid {
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 2rem;
        }

        @media (max-width: 1024px) {
            .dashboard-grid {
                grid-template-columns: 1fr;
            }
        }

        .panel {
            background-color: var(--panel-bg);
            border: 1px solid var(--border-color);
            border-radius: 12px;
            padding: 1.5rem;
            margin-bottom: 2rem;
        }

        .panel-title {
            font-size: 1.25rem;
            font-weight: 700;
            margin-bottom: 1.2rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            color: var(--accent-secondary);
        }

        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.9rem;
        }

        th, td {
            padding: 0.75rem 1rem;
            text-align: left;
            border-bottom: 1px solid var(--border-color);
        }

        th {
            color: var(--text-muted);
            font-weight: 600;
            text-transform: uppercase;
            font-size: 0.75rem;
            letter-spacing: 0.5px;
        }

        td {
            font-family: 'JetBrains Mono', monospace;
        }

        tr:last-child td {
            border-bottom: none;
        }

        .mono {
            font-family: 'JetBrains Mono', monospace;
        }

        .badge {
            display: inline-block;
            padding: 0.2rem 0.5rem;
            border-radius: 4px;
            font-size: 0.75rem;
            font-weight: 700;
        }

        .badge-success { background-color: rgba(16, 185, 129, 0.2); color: var(--success); }
        .badge-danger { background-color: rgba(239, 68, 68, 0.2); color: var(--danger); }
        .badge-info { background-color: rgba(6, 182, 212, 0.2); color: var(--accent-secondary); }
        .badge-warning { background-color: rgba(245, 158, 11, 0.2); color: var(--warning); }

        .error-message {
            color: var(--danger);
            font-size: 0.8rem;
            word-break: break-all;
        }

        .btn {
            background-color: var(--panel-bg);
            color: var(--text-color);
            border: 1px solid var(--border-color);
            padding: 0.3rem 0.6rem;
            text-decoration: none;
            border-radius: 4px;
            font-size: 0.8rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
        }

        .btn:hover {
            border-color: var(--accent-primary);
            color: var(--accent-primary);
        }

        .btn-danger {
            color: var(--danger);
            border-color: rgba(239, 68, 68, 0.2);
            background-color: rgba(239, 68, 68, 0.05);
        }

        .btn-danger:hover {
            color: #fff;
            background-color: var(--danger);
            border-color: var(--danger);
        }
        
        .btn-warning {
            color: var(--warning);
            border-color: rgba(245, 158, 11, 0.2);
            background-color: rgba(245, 158, 11, 0.05);
        }

        .btn-warning:hover {
            color: #fff;
            background-color: var(--warning);
            border-color: var(--warning);
        }

        .key-cell {
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }
        .key-text {
            color: var(--accent-secondary);
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.82rem;
            letter-spacing: 0.03em;
            cursor: default;
        }
        .btn-copy {
            flex-shrink: 0;
            padding: 0.15rem 0.5rem;
            font-size: 0.72rem;
        }
        .btn-copy.copied {
            color: var(--success);
            border-color: var(--success);
        }
        .username-cell {
            font-family: 'Outfit', sans-serif;
            font-weight: 600;
            color: #fff;
        }
    </style>
</head>
<body>
    <header>
        <h1>Questline Cloud Chronicle</h1>
        <div style="display: flex; gap: 1rem; align-items: center;">
            <form method="POST" onsubmit="return confirm('WARNING: This will permanently wipe all users, backups, messages, and logs. Proceed?');" style="margin: 0;">
                <input type="hidden" name="action" value="reset_all">
                <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                <button type="submit" class="btn btn-danger" style="padding: 0.5rem 1rem; border-radius: 6px;">Reset Database</button>
            </form>
            <a href="?logout=1">Sign Out</a>
        </div>
    </header>

    <?php if (isset($_GET['msg']) && !empty($_GET['msg'])): ?>
        <div style="background-color: rgba(16, 185, 129, 0.2); color: var(--success); border: 1px solid var(--success); padding: 1rem; border-radius: 8px; margin-bottom: 2rem; font-weight: 600;">
            <?= htmlspecialchars($_GET['msg']) ?>
        </div>
    <?php endif; ?>
    <?php if (isset($_GET['err']) && !empty($_GET['err'])): ?>
        <div style="background-color: rgba(239, 68, 68, 0.2); color: var(--danger); border: 1px solid var(--danger); padding: 1rem; border-radius: 8px; margin-bottom: 2rem; font-weight: 600;">
            <?= htmlspecialchars($_GET['err']) ?>
        </div>
    <?php endif; ?>

    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-value"><?= $users_count ?></div>
            <div class="stat-label">Total Users</div>
        </div>
        <div class="stat-card cyan">
            <div class="stat-value"><?= $devices_count ?></div>
            <div class="stat-label">Active Devices</div>
        </div>
        <div class="stat-card success">
            <div class="stat-value"><?= $sync_queue_count ?></div>
            <div class="stat-label">Sync Events</div>
        </div>
        <div class="stat-card">
            <div class="stat-value"><?= $messages_count ?></div>
            <div class="stat-label">Chronicle Messages</div>
        </div>
        <div class="stat-card cyan">
            <div class="stat-value"><?= $backups_count ?></div>
            <div class="stat-label">Total Backups</div>
        </div>
        <div class="stat-card danger">
            <div class="stat-value"><?= $errors_count ?></div>
            <div class="stat-label">API Error Logs</div>
        </div>
    </div>

    <!-- Directorio de usuarios — ancho completo para que quepa la llave pública -->
    <div class="panel">
        <div class="panel-title">
            Users Directory
            <span style="font-size:0.8rem; font-weight:400; color:var(--text-muted);"><?= $users_count ?> registered</span>
        </div>
        <table>
            <thead>
                <tr>
                    <th style="width:18%;">Character Name</th>
                    <th>Public Key — Share to Invite</th>
                    <th style="width:14%;">Registered</th>
                    <th style="text-align:right; width:8%;">Actions</th>
                </tr>
            </thead>
            <tbody>
                <?php if (empty($users)): ?>
                    <tr><td colspan="4" style="text-align:center; color:var(--text-muted)">No users registered yet.</td></tr>
                <?php else: ?>
                    <?php foreach ($users as $u): ?>
                        <tr>
                            <td class="username-cell"><?= htmlspecialchars($u['username']) ?></td>
                            <td>
                                <div class="key-cell">
                                    <span class="key-text" title="<?= htmlspecialchars($u['public_key']) ?>">
                                        <?= substr($u['public_key'], 0, 24) ?>…<?= substr($u['public_key'], -8) ?>
                                    </span>
                                    <button class="btn btn-copy"
                                        data-key="<?= htmlspecialchars($u['public_key']) ?>"
                                        onclick="copyKey(this)">Copy</button>
                                </div>
                            </td>
                            <td><?= date('Y-m-d H:i', strtotime($u['created_at'])) ?></td>
                            <td style="text-align:right;">
                                <form method="POST" style="display:inline;" onsubmit="return confirm('Delete this user?');">
                                    <input type="hidden" name="action" value="delete_user">
                                    <input type="hidden" name="user_id" value="<?= htmlspecialchars($u['id']) ?>">
                                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                                    <button type="submit" class="btn btn-danger">Delete</button>
                                </form>
                            </td>
                        </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
            </tbody>
        </table>
    </div>

    <div class="dashboard-grid">
        <!-- Panel 2: Dispositivos registrados — útil para ver qué devices están activos -->
        <div class="panel">
            <div class="panel-title">Device Mesh</div>
            <table>
                <thead>
                    <tr>
                        <th>Device Name</th>
                        <th>User Identity</th>
                        <th>Last Seen</th>
                    </tr>
                </thead>
                <tbody>
                    <?php if (empty($devices)): ?>
                        <tr><td colspan="3" style="text-align: center; color: var(--text-muted)">No active devices.</td></tr>
                    <?php else: ?>
                        <?php foreach ($devices as $d): ?>
                            <tr>
                                <td style="color: #fff; font-weight: 600;"><?= htmlspecialchars($d['device_name']) ?></td>
                                <td title="<?= $d['public_key'] ?>"><?= substr($d['public_key'], 0, 12) ?>...</td>
                                <td><?= date('Y-m-d H:i:s', strtotime($d['last_seen'])) ?></td>
                            </tr>
                        <?php endforeach; ?>
                    <?php endif; ?>
                </tbody>
            </table>
        </div>

        <!-- Panel 3: Cola de sync — si está muy llena algo anda mal con los clientes -->
        <div class="panel">
            <div class="panel-title">Sync events queue</div>
            <table>
                <thead>
                    <tr>
                        <th>Entity Type</th>
                        <th>Entity ID</th>
                        <th>Operation</th>
                        <th>Timestamp</th>
                    </tr>
                </thead>
                <tbody>
                    <?php if (empty($sync_events)): ?>
                        <tr><td colspan="4" style="text-align: center; color: var(--text-muted)">Sync queue empty.</td></tr>
                    <?php else: ?>
                        <?php foreach ($sync_events as $se): ?>
                            <tr>
                                <td><span class="badge badge-info"><?= strtoupper($se['entity_type']) ?></span></td>
                                <td title="<?= htmlspecialchars($se['entity_id']) ?>"><?= substr($se['entity_id'], 0, 8) ?>...</td>
                                <td><?= htmlspecialchars($se['operation']) ?></td>
                                <td><?= htmlspecialchars($se['created_at']) ?></td>
                            </tr>
                        <?php endforeach; ?>
                    <?php endif; ?>
                </tbody>
            </table>
        </div>

        <!-- Panel 4: Mensajes del chronicle — chats de los proyectos compartidos -->
        <div class="panel">
            <div class="panel-title">Fellowship Chronicle Chat Logs</div>
            <table>
                <thead>
                    <tr>
                        <th>Fellowship ID</th>
                        <th>Sender</th>
                        <th>Content</th>
                        <th>Timestamp</th>
                    </tr>
                </thead>
                <tbody>
                    <?php if (empty($messages)): ?>
                        <tr><td colspan="4" style="text-align: center; color: var(--text-muted)">No chat messages posted.</td></tr>
                    <?php else: ?>
                        <?php foreach ($messages as $m): ?>
                            <tr>
                                <td title="<?= $m['project_id'] ?>"><?= substr($m['project_id'], 0, 8) ?>...</td>
                                <td style="color: var(--accent-primary)"><?= htmlspecialchars($m['sender_username']) ?></td>
                                <td><?= htmlspecialchars($m['content']) ?></td>
                                <td><?= htmlspecialchars($m['timestamp']) ?></td>
                            </tr>
                        <?php endforeach; ?>
                    <?php endif; ?>
                </tbody>
            </table>
        </div>
    </div>

    <!-- Panel 5: Backups de usuarios — se puede ver el tamaño y borrar si es necesario -->
    <div class="panel">
        <div class="panel-title">Database Backups</div>
        <table>
            <thead>
                <tr>
                    <th>User Identity (UUID)</th>
                    <th>User Public Key</th>
                    <th>Backup Size</th>
                    <th>Last Updated</th>
                    <th style="text-align: right;">Actions</th>
                </tr>
            </thead>
            <tbody>
                <?php if (empty($backups)): ?>
                    <tr><td colspan="5" style="text-align: center; color: var(--text-muted)">No backups found.</td></tr>
                <?php else: ?>
                    <?php foreach ($backups as $b): ?>
                        <tr>
                            <td title="<?= htmlspecialchars($b['user_id']) ?>"><?= substr($b['user_id'], 0, 8) ?>...</td>
                            <td title="<?= htmlspecialchars($b['public_key']) ?>" style="color: var(--accent-secondary)"><?= substr($b['public_key'], 0, 16) ?>...</td>
                            <td><?= round($b['size'] / 1024, 2) ?> KB</td>
                            <td><?= date('Y-m-d H:i', strtotime($b['created_at'])) ?></td>
                            <td style="text-align: right;">
                                <form method="POST" style="display:inline;" onsubmit="return confirm('Are you sure you want to delete this backup?');">
                                    <input type="hidden" name="action" value="delete_backup">
                                    <input type="hidden" name="user_id" value="<?= htmlspecialchars($b['user_id']) ?>">
                                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                                    <button type="submit" class="btn btn-danger">Delete</button>
                                </form>
                            </td>
                        </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
            </tbody>
        </table>
    </div>

    <!-- Panel 6: Logs de seguridad y errores — aquí se ven auth failures y errores de API -->
    <div class="panel">
        <div class="panel-title" style="display: flex; justify-content: space-between; align-items: center;">
            <span>API security, auth, and error logs</span>
            <form method="POST" style="margin:0;" onsubmit="return confirm('Clear all log entries?');">
                <input type="hidden" name="action" value="clear_logs">
                <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                <button type="submit" class="btn btn-warning">Clear Logs</button>
            </form>
        </div>
        <table>
            <thead>
                <tr>
                    <th>Log Type</th>
                    <th>Error Detail</th>
                    <th>Timestamp</th>
                </tr>
            </thead>
            <tbody>
                <?php if (empty($error_logs)): ?>
                    <tr><td colspan="3" style="text-align: center; color: var(--text-muted)">No errors logged.</td></tr>
                <?php else: ?>
                    <?php foreach ($error_logs as $log): ?>
                        <tr>
                            <td>
                                <span class="badge <?= $log['log_type'] === 'AUTH_FAILURE' ? 'badge-danger' : ($log['log_type'] === 'API_ERROR' ? 'badge-warning' : 'badge-info') ?>">
                                    <?= htmlspecialchars($log['log_type']) ?>
                                </span>
                            </td>
                            <td class="error-message"><?= htmlspecialchars($log['message']) ?></td>
                            <td><?= date('Y-m-d H:i:s', strtotime($log['created_at'])) ?></td>
                        </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
            </tbody>
        </table>
    </div>
    <script>
        function copyKey(btn) {
            const key = btn.dataset.key;
            navigator.clipboard.writeText(key).then(() => {
                btn.textContent = 'Copied!';
                btn.classList.add('copied');
                setTimeout(() => {
                    btn.textContent = 'Copy';
                    btn.classList.remove('copied');
                }, 1800);
            });
        }
    </script>
</body>
</html>
