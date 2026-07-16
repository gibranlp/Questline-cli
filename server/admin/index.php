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
        } elseif ($action === 'generate_access_code') {
            $label     = substr(trim($_POST['label'] ?? ''), 0, 255);
            $cliPubKey = trim($_POST['cli_public_key'] ?? '');
            $linkedId  = null;
            if (strlen($cliPubKey) === 64 && ctype_xdigit($cliPubKey)) {
                $stmt = $pdo->prepare("SELECT id FROM users WHERE public_key = ?");
                $stmt->execute([$cliPubKey]);
                $found = $stmt->fetchColumn();
                if ($found) {
                    $linkedId = $found;
                } else {
                    header("Location: " . $_SERVER['PHP_SELF'] . "?err=" . urlencode("CLI public key not found — user must sync their CLI at least once before linking."));
                    exit;
                }
            }
            $code = strtoupper(bin2hex(random_bytes(8)));
            $code = implode('-', str_split($code, 4));
            $stmt = $pdo->prepare("INSERT INTO access_codes (code, label, created_by, linked_user_id) VALUES (?, ?, 'admin', ?)");
            $stmt->execute([$code, $label ?: null, $linkedId]);
            $success_msg = "Access code generated: $code" . ($linkedId ? " (linked to existing CLI account)" : '');
        } elseif ($action === 'revoke_access_code') {
            $code = trim($_POST['code'] ?? '');
            if (!empty($code)) {
                $stmt = $pdo->prepare("UPDATE access_codes SET redeemed_by_user_id = 'REVOKED', redeemed_at = CURRENT_TIMESTAMP WHERE code = ?");
                $stmt->execute([$code]);
                $success_msg = "Access code revoked.";
            }
        } elseif ($action === 'toggle_supporter') {
            $uid  = trim($_POST['user_id'] ?? '');
            $flag = intval($_POST['supporter'] ?? 0);
            if (!empty($uid)) {
                $stmt = $pdo->prepare("UPDATE users SET supporter = ? WHERE id = ?");
                $stmt->execute([$flag, $uid]);
                $success_msg = "Supporter status updated.";
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

        // ── CRUD del Lore Library — lee/escribe los JSON en server/data/ ─────────

        } elseif (in_array($action, ['lore_add','lore_edit','lore_delete','quest_add','quest_edit','quest_delete'])) {

            $LORE_FILE   = dirname(__DIR__) . '/data/lore.json';
            $QUESTS_FILE = dirname(__DIR__) . '/data/quests.json';

            // Construye el objeto unlock desde los campos del formulario
            $build_unlock = function() {
                $type = $_POST['unlock_type'] ?? 'free';
                $u = ['type' => $type, 'display' => trim($_POST['unlock_display'] ?? '')];
                if ($type === 'level' || $type === 'class_level') {
                    $u['level'] = (int)($_POST['unlock_level'] ?? 0);
                }
                if ($type === 'class_level') {
                    $u['class'] = trim($_POST['unlock_class'] ?? '');
                }
                if ($type === 'milestone') {
                    $u['milestone_id'] = trim($_POST['unlock_milestone_id'] ?? '');
                }
                if ($type === 'chapter_reward') {
                    $u['chapter_id'] = trim($_POST['unlock_chapter_id'] ?? '');
                }
                return $u;
            };

            if ($action === 'lore_add' || $action === 'lore_edit') {
                $raw  = file_get_contents($LORE_FILE);
                $data = json_decode($raw, true);
                $entries = &$data['entries'];

                $id         = trim($_POST['entry_id'] ?? '');
                $category   = trim($_POST['category'] ?? '');
                $title      = trim($_POST['title'] ?? '');
                $content    = str_replace("\r\n", "\n", $_POST['content'] ?? '');
                $class_filt = trim($_POST['class_filter'] ?? '') ?: null;
                $rarity     = trim($_POST['rarity'] ?? '') ?: null;
                $sort_order = (int)($_POST['sort_order'] ?? 0);

                if (!$id || !$category || !$title) {
                    throw new Exception("ID, Category y Title son obligatorios.");
                }

                $entry = [
                    'id'          => $id,
                    'category'    => $category,
                    'title'       => $title,
                    'content'     => $content,
                    'class_filter'=> $class_filt,
                    'unlock'      => $build_unlock(),
                    'rarity'      => $rarity,
                    'sort_order'  => $sort_order,
                ];

                if ($action === 'lore_add') {
                    // Verifica duplicados
                    foreach ($entries as $e) {
                        if ($e['id'] === $id) throw new Exception("Ya existe una entrada con ID '$id'.");
                    }
                    $entries[] = $entry;
                    $success_msg = "Lore entry '$id' added.";
                } else {
                    // Reemplaza la entrada existente
                    $found = false;
                    foreach ($entries as &$e) {
                        if ($e['id'] === $id) { $e = $entry; $found = true; break; }
                    }
                    unset($e);
                    if (!$found) throw new Exception("Entry '$id' not found.");
                    $success_msg = "Lore entry '$id' updated.";
                }

                // Ordena por sort_order para mantener el JSON legible
                usort($data['entries'], fn($a,$b) => ($a['sort_order'] ?? 0) <=> ($b['sort_order'] ?? 0));
                file_put_contents($LORE_FILE, json_encode($data, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES));

            } elseif ($action === 'lore_delete') {
                $id   = trim($_POST['entry_id'] ?? '');
                $raw  = file_get_contents($LORE_FILE);
                $data = json_decode($raw, true);
                $before = count($data['entries']);
                $data['entries'] = array_values(array_filter($data['entries'], fn($e) => $e['id'] !== $id));
                if (count($data['entries']) === $before) throw new Exception("Entry '$id' not found.");
                file_put_contents($LORE_FILE, json_encode($data, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES));
                $success_msg = "Lore entry '$id' deleted.";

            } elseif ($action === 'quest_add' || $action === 'quest_edit') {
                $raw  = file_get_contents($QUESTS_FILE);
                $data = json_decode($raw, true);
                $quests = &$data['quests'];

                $class    = trim($_POST['quest_class'] ?? '');
                $level    = (int)($_POST['quest_level'] ?? 0);
                $name     = trim($_POST['quest_name'] ?? '');
                $desc     = trim($_POST['quest_description'] ?? '');
                $obj_type = trim($_POST['obj_type'] ?? '');
                $obj_tgt  = (int)($_POST['obj_target'] ?? 0);
                $reward   = trim($_POST['lore_reward'] ?? '');
                $reward_id= trim($_POST['reward_lore_id'] ?? '');

                if (!$class || !$level || !$name) throw new Exception("Class, Level y Name son obligatorios.");

                $quest = [
                    'class'         => $class,
                    'level'         => $level,
                    'name'          => $name,
                    'description'   => $desc,
                    'objective'     => ['type' => $obj_type, 'target' => $obj_tgt],
                    'lore_reward'   => $reward,
                    'reward_lore_id'=> $reward_id,
                ];

                if ($action === 'quest_add') {
                    foreach ($quests as $q) {
                        if ($q['class'] === $class && $q['level'] === $level) {
                            throw new Exception("Ya existe una quest para $class nivel $level.");
                        }
                    }
                    $quests[] = $quest;
                    $success_msg = "Quest '$name' added.";
                } else {
                    $orig_class = trim($_POST['orig_class'] ?? $class);
                    $orig_level = (int)($_POST['orig_level'] ?? $level);
                    $found = false;
                    foreach ($quests as &$q) {
                        if ($q['class'] === $orig_class && $q['level'] === $orig_level) {
                            $q = $quest; $found = true; break;
                        }
                    }
                    unset($q);
                    if (!$found) throw new Exception("Quest not found.");
                    $success_msg = "Quest '$name' updated.";
                }

                usort($data['quests'], fn($a,$b) => $a['class'] <=> $b['class'] ?: $a['level'] <=> $b['level']);
                file_put_contents($QUESTS_FILE, json_encode($data, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES));

            } elseif ($action === 'quest_delete') {
                $class = trim($_POST['quest_class'] ?? '');
                $level = (int)($_POST['quest_level'] ?? 0);
                $raw   = file_get_contents($QUESTS_FILE);
                $data  = json_decode($raw, true);
                $before = count($data['quests']);
                $data['quests'] = array_values(array_filter($data['quests'], fn($q) => !($q['class'] === $class && $q['level'] === $level)));
                if (count($data['quests']) === $before) throw new Exception("Quest not found.");
                file_put_contents($QUESTS_FILE, json_encode($data, JSON_PRETTY_PRINT | JSON_UNESCAPED_UNICODE | JSON_UNESCAPED_SLASHES));
                $success_msg = "Quest deleted.";
            }
        }

        header("Location: " . $_SERVER['PHP_SELF'] . "?msg=" . urlencode($success_msg));
        exit;
    } catch (Exception $e) {
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

// Usuarios con nombre resuelto — users.username primero, luego project_members, luego chronicle
$users = $pdo->query("
    SELECT
        u.id,
        u.public_key,
        u.created_at,
        COALESCE(
            NULLIF(u.username, ''),
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

// Access codes para el panel de webapp
try {
    $access_codes = $pdo->query("SELECT code, label, created_by, created_at, redeemed_by_user_id, redeemed_at, linked_user_id FROM access_codes ORDER BY created_at DESC LIMIT 100")->fetchAll();
    $supporters = $pdo->query("
        SELECT u.id, u.public_key, u.email, u.supporter, u.created_at,
               wa.username
        FROM users u
        LEFT JOIN webapp_accounts wa ON wa.user_id = u.id
        WHERE u.supporter = 1
        ORDER BY u.created_at DESC LIMIT 50
    ")->fetchAll();
} catch (PDOException $e) {
    $access_codes = [];
    $supporters = [];
}

// ── Funciones helper para renderizar los formularios de lore ──────────────────
function _lore_sel(array $opts, string $cur): string {
    $out = '';
    foreach ($opts as $o) {
        $val   = htmlspecialchars($o);
        $label = htmlspecialchars($o ?: '(none)');
        $sel   = ($o === $cur) ? ' selected' : '';
        $out  .= "<option value=\"$val\"$sel>$label</option>";
    }
    return $out;
}

function _lore_input(string $name, string $val, string $type='text', string $extra=''): string {
    $s = 'background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;';
    return "<input type=\"$type\" name=\"$name\" value=\"" . htmlspecialchars($val) . "\" style=\"$s\" $extra>";
}

function _lore_field(string $label, string $input_html): string {
    return '<div style="display:flex;flex-direction:column;gap:.25rem;margin-bottom:.75rem;">'
         . '<label style="font-size:.75rem;color:var(--text-muted)">' . $label . '</label>'
         . $input_html . '</div>';
}

function lore_entry_form_fields(?array $e): string {
    $u     = $e['unlock'] ?? [];
    $utype = $u['type'] ?? 'free';
    $id    = $e['id'] ?? '';

    $classes   = ['Code Warlock','Task Paladin','Mind Sage','Systems Architect','Time Chronomancer','Arch Accountant'];
    $categories= ['World','Class','Memory','Achievement'];
    $utypes    = ['free','level','class_level','discovery','chapter_reward','milestone'];
    $rarities  = ['','common','rare','legendary'];

    $sel_input = 'background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;';

    $lv_disp  = in_array($utype, ['level','class_level']) ? '' : 'display:none;';
    $cls_disp = ($utype === 'class_level')  ? '' : 'display:none;';
    $mil_disp = ($utype === 'milestone')    ? '' : 'display:none;';
    $ch_disp  = ($utype === 'chapter_reward') ? '' : 'display:none;';

    $readonly = $e ? 'readonly' : '';

    ob_start(); ?>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:1rem;">
        <div>
            <?= _lore_field('ID *',
                "<input type=\"text\" name=\"entry_id\" value=\"" . htmlspecialchars($id) . "\" required $readonly
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;font-size:.85rem;\">") ?>
            <?= _lore_field('Category *',
                "<select name=\"category\" style=\"$sel_input\">" . _lore_sel($categories, $e['category'] ?? '') . "</select>") ?>
            <?= _lore_field('Title *',
                "<input type=\"text\" name=\"title\" value=\"" . htmlspecialchars($e['title'] ?? '') . "\" required
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            <?= _lore_field('Class Filter',
                "<select name=\"class_filter\" style=\"$sel_input\">" . _lore_sel(array_merge([''], $classes), $e['class_filter'] ?? '') . "</select>") ?>
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:.5rem;">
                <?= _lore_field('Rarity',
                    "<select name=\"rarity\" style=\"$sel_input\">" . _lore_sel($rarities, $e['rarity'] ?? '') . "</select>") ?>
                <?= _lore_field('Sort Order',
                    "<input type=\"number\" name=\"sort_order\" value=\"" . intval($e['sort_order'] ?? 0) . "\"
                     style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            </div>
        </div>
        <div>
            <?= _lore_field('Unlock Type *',
                "<select name=\"unlock_type\" onchange=\"updateUnlockFields(this)\" style=\"$sel_input\">"
                . _lore_sel($utypes, $utype) . "</select>") ?>
            <div class="lore-field-row" style="<?= $lv_disp ?>">
                <?= _lore_field('Unlock Level',
                    "<input type=\"number\" id=\"unlock_level\" name=\"unlock_level\" value=\"" . intval($u['level'] ?? 0) . "\"
                     style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            </div>
            <div class="lore-field-row" style="<?= $cls_disp ?>">
                <?= _lore_field('Unlock Class',
                    "<select id=\"unlock_class\" name=\"unlock_class\" style=\"$sel_input\">"
                    . _lore_sel(array_merge([''], $classes), $u['class'] ?? '') . "</select>") ?>
            </div>
            <div class="lore-field-row" style="<?= $mil_disp ?>">
                <?= _lore_field('Milestone ID',
                    "<input type=\"text\" id=\"unlock_milestone_id\" name=\"unlock_milestone_id\" value=\"" . htmlspecialchars($u['milestone_id'] ?? '') . "\"
                     style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            </div>
            <div class="lore-field-row" style="<?= $ch_disp ?>">
                <?= _lore_field('Chapter ID',
                    "<input type=\"text\" id=\"unlock_chapter_id\" name=\"unlock_chapter_id\" value=\"" . htmlspecialchars($u['chapter_id'] ?? '') . "\"
                     style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            </div>
            <?= _lore_field('Unlock Display Text',
                "<input type=\"text\" name=\"unlock_display\" value=\"" . htmlspecialchars($u['display'] ?? '') . "\"
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
        </div>
    </div>
    <div style="display:flex;flex-direction:column;gap:.25rem;margin-bottom:.75rem;margin-top:.25rem;">
        <label style="font-size:.75rem;color:var(--text-muted)">Content</label>
        <textarea name="content" rows="8"
            style="background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.6rem .8rem;border-radius:4px;font-family:inherit;font-size:.82rem;resize:vertical;"><?= htmlspecialchars($e['content'] ?? '') ?></textarea>
    </div>
    <?php return ob_get_clean();
}

function quest_form_fields(?array $q): string {
    $classes  = ['Code Warlock','Task Paladin','Mind Sage','Systems Architect','Time Chronomancer','Arch Accountant'];
    $obj_types= ['tasks_completed','focus_minutes','zen_waterings','projects_completed','streak_days'];

    $sel_input = 'background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;';

    ob_start(); ?>
    <div style="display:grid;grid-template-columns:1fr 1fr;gap:1rem;">
        <div>
            <?= _lore_field('Class *',
                "<select name=\"quest_class\" style=\"$sel_input\">" . _lore_sel($classes, $q['class'] ?? '') . "</select>") ?>
            <?= _lore_field('Quest Level *',
                "<input type=\"number\" name=\"quest_level\" value=\"" . intval($q['level'] ?? 10) . "\" min=\"1\" max=\"100\" required
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            <?= _lore_field('Name *',
                "<input type=\"text\" name=\"quest_name\" value=\"" . htmlspecialchars($q['name'] ?? '') . "\" required
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            <?= _lore_field('Lore Reward (flavor text)',
                "<input type=\"text\" name=\"lore_reward\" value=\"" . htmlspecialchars($q['lore_reward'] ?? '') . "\"
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            <?= _lore_field('Reward Lore ID',
                "<input type=\"text\" name=\"reward_lore_id\" value=\"" . htmlspecialchars($q['reward_lore_id'] ?? '') . "\" placeholder=\"e.g. quest_lore_10\"
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
        </div>
        <div>
            <?= _lore_field('Objective Type',
                "<select name=\"obj_type\" style=\"$sel_input\">" . _lore_sel($obj_types, $q['objective']['type'] ?? '') . "</select>") ?>
            <?= _lore_field('Objective Target',
                "<input type=\"number\" name=\"obj_target\" value=\"" . intval($q['objective']['target'] ?? 10) . "\" min=\"1\"
                 style=\"background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.4rem .6rem;border-radius:4px;font-family:inherit;\">") ?>
            <div style="display:flex;flex-direction:column;gap:.25rem;margin-bottom:.75rem;margin-top:.5rem;">
                <label style="font-size:.75rem;color:var(--text-muted)">Description</label>
                <textarea name="quest_description" rows="7"
                    style="background:var(--bg);border:1px solid var(--border);color:var(--text);padding:.6rem .8rem;border-radius:4px;font-family:inherit;font-size:.82rem;resize:vertical;"><?= htmlspecialchars($q['description'] ?? '') ?></textarea>
            </div>
        </div>
    </div>
    <?php return ob_get_clean();
}

// ── Datos de lore — leídos directamente de los JSON en server/data/ ───────────
$LORE_FILE   = dirname(__DIR__) . '/data/lore.json';
$QUESTS_FILE = dirname(__DIR__) . '/data/quests.json';

$lore_entries = [];
$lore_quests  = [];

if (file_exists($LORE_FILE)) {
    $raw = file_get_contents($LORE_FILE);
    $decoded = json_decode($raw, true);
    $lore_entries = $decoded['entries'] ?? [];
}
if (file_exists($QUESTS_FILE)) {
    $raw = file_get_contents($QUESTS_FILE);
    $decoded = json_decode($raw, true);
    $lore_quests = $decoded['quests'] ?? [];
}

// Parámetros de edición vía GET — para pre-rellenar formularios
$edit_lore_id    = $_GET['edit_lore']  ?? null;
$edit_quest_key  = $_GET['edit_quest'] ?? null; // "Class|Level"

$edit_lore_entry = null;
if ($edit_lore_id) {
    foreach ($lore_entries as $e) {
        if ($e['id'] === $edit_lore_id) { $edit_lore_entry = $e; break; }
    }
}
$edit_quest_entry = null;
if ($edit_quest_key) {
    [$eq_class, $eq_level] = explode('|', $edit_quest_key, 2) + ['', '0'];
    foreach ($lore_quests as $q) {
        if ($q['class'] === $eq_class && (string)$q['level'] === $eq_level) {
            $edit_quest_entry = $q; break;
        }
    }
}

?>
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Questline Cloud Chronicle Admin Dashboard</title>
    <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --warlock:    rgb(168,  85, 247);
            --paladin:    rgb(255, 105, 180);
            --sage:       rgb(  6, 182, 212);
            --architect:  rgb( 59, 130, 246);
            --chrono:     rgb(249, 115,  22);
            --accountant: rgb(245, 158,  11);

            --bg:         #080808;
            --bg-card:    #0f0f0f;
            --bg-border:  #1c1c1c;
            --text:       #d4d4d4;
            --text-dim:   #999;
            --text-dimmer:#2a2a2a;

            --success:    #10b981;
            --warning:    var(--accountant);
            --danger:     #ef4444;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
        }

        /* scanline overlay */
        body::before {
            content: '';
            pointer-events: none;
            position: fixed;
            inset: 0;
            background: repeating-linear-gradient(
                0deg, transparent, transparent 2px,
                rgba(0,0,0,0.08) 2px, rgba(0,0,0,0.08) 4px
            );
            z-index: 9999;
        }

        body {
            background-color: var(--bg);
            color: var(--text);
            font-family: 'JetBrains Mono', 'Courier New', Courier, monospace;
            line-height: 1.6;
            padding: 2rem;
        }

        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 2rem;
            border-bottom: 1px solid var(--bg-border);
            padding-bottom: 1rem;
        }

        header h1 {
            font-size: 1.5rem;
            font-weight: 700;
            letter-spacing: 0.1em;
            text-transform: uppercase;
            animation: cycleColor 9s linear infinite;
        }

        @keyframes cycleColor {
            0%   { color: var(--warlock);    text-shadow: 0 0 15px rgba(168, 85, 247, 0.4);    }
            16%  { color: var(--paladin);    text-shadow: 0 0 15px rgba(255, 105, 180, 0.4);    }
            33%  { color: var(--sage);       text-shadow: 0 0 15px rgba(6, 182, 212, 0.4);       }
            50%  { color: var(--architect);  text-shadow: 0 0 15px rgba(59, 130, 246, 0.4);  }
            66%  { color: var(--chrono);     text-shadow: 0 0 15px rgba(249, 115, 22, 0.4);     }
            83%  { color: var(--accountant); text-shadow: 0 0 15px rgba(245, 158, 11, 0.4); }
            100% { color: var(--warlock);    text-shadow: 0 0 15px rgba(168, 85, 247, 0.4);    }
        }

        header a {
            background-color: var(--bg-card);
            color: var(--text);
            border: 1px solid var(--bg-border);
            padding: 0.5rem 1rem;
            text-decoration: none;
            font-weight: 600;
            transition: all 0.2s ease;
        }

        header a:hover {
            border-color: var(--warlock);
            color: #fff;
            box-shadow: 0 0 10px rgba(168, 85, 247, 0.3);
        }

        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2.5rem;
        }

        .stat-card {
            background-color: var(--bg-card);
            border: 1px solid var(--bg-border);
            padding: 1.5rem;
            text-align: center;
            position: relative;
            transition: transform 0.2s ease, border-color 0.2s;
        }

        .stat-card:hover {
            transform: translateY(-2px);
            border-color: var(--text-dimmer);
        }

        .stat-card::before {
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 2px;
            background-color: var(--warlock);
        }

        .stat-card.cyan::before {
            background-color: var(--sage);
        }

        .stat-card.danger::before {
            background-color: var(--danger);
        }

        .stat-card.success::before {
            background-color: var(--success);
        }

        .stat-value {
            font-size: 2.2rem;
            font-weight: 700;
            margin: 0.5rem 0;
            color: #fff;
        }

        .stat-label {
            color: var(--text-dim);
            text-transform: uppercase;
            font-size: 0.75rem;
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
            background-color: var(--bg-card);
            border: 1px solid var(--bg-border);
            padding: 1.5rem;
            margin-bottom: 2rem;
        }

        .panel-title {
            font-size: 1.1rem;
            font-weight: 700;
            margin-bottom: 1.2rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            color: var(--sage);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            font-size: 0.85rem;
        }

        th, td {
            padding: 0.75rem 1rem;
            text-align: left;
            border-bottom: 1px solid var(--bg-border);
        }

        th {
            color: var(--text-dim);
            font-weight: 600;
            text-transform: uppercase;
            font-size: 0.72rem;
            letter-spacing: 0.5px;
        }

        td {
            color: var(--text);
        }

        tr:last-child td {
            border-bottom: none;
        }

        .mono {
            font-family: inherit;
        }

        .badge {
            display: inline-block;
            padding: 0.15rem 0.4rem;
            border-radius: 2px;
            font-size: 0.7rem;
            font-weight: 700;
            text-transform: uppercase;
        }

        .badge-success { background-color: rgba(16, 185, 129, 0.15); color: var(--success); border: 1px solid rgba(16, 185, 129, 0.3); }
        .badge-danger { background-color: rgba(239, 68, 68, 0.15); color: var(--danger); border: 1px solid rgba(239, 68, 68, 0.3); }
        .badge-info { background-color: rgba(6, 182, 212, 0.15); color: var(--sage); border: 1px solid rgba(6, 182, 212, 0.3); }
        .badge-warning { background-color: rgba(245, 158, 11, 0.15); color: var(--warning); border: 1px solid rgba(245, 158, 11, 0.3); }

        .error-message {
            color: var(--danger);
            font-size: 0.8rem;
            word-break: break-all;
        }

        .btn {
            background-color: var(--bg-card);
            color: var(--text);
            border: 1px solid var(--bg-border);
            padding: 0.3rem 0.6rem;
            text-decoration: none;
            font-size: 0.8rem;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.2s ease;
        }

        .btn:hover {
            border-color: var(--warlock);
            color: #fff;
            box-shadow: 0 0 8px rgba(168, 85, 247, 0.2);
        }

        .btn-danger {
            color: var(--danger);
            border-color: rgba(239, 68, 68, 0.3);
            background-color: rgba(239, 68, 68, 0.05);
        }

        .btn-danger:hover {
            color: #fff;
            background-color: var(--danger);
            border-color: var(--danger);
            box-shadow: 0 0 8px rgba(239, 68, 68, 0.4);
        }
        
        .btn-warning {
            color: var(--warning);
            border-color: rgba(245, 158, 11, 0.3);
            background-color: rgba(245, 158, 11, 0.05);
        }

        .btn-warning:hover {
            color: #fff;
            background-color: var(--warning);
            border-color: var(--warning);
            box-shadow: 0 0 8px rgba(245, 158, 11, 0.4);
        }

        .key-cell {
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }
        .key-text {
            color: var(--sage);
            font-size: 0.8rem;
            letter-spacing: 0.03em;
            cursor: default;
        }
        .btn-copy {
            flex-shrink: 0;
            padding: 0.15rem 0.5rem;
            font-size: 0.7rem;
        }
        .btn-copy.copied {
            color: var(--success);
            border-color: var(--success);
        }
        .username-cell {
            font-weight: 600;
            color: #fff;
        }

        /* ── Tab navigation ────────────────────────────────────────────────── */
        .tab-nav {
            display: flex;
            gap: 0.25rem;
            margin-bottom: 1.5rem;
            border-bottom: 1px solid var(--border);
            padding-bottom: 0;
            position: sticky;
            top: 0;
            background: var(--bg);
            z-index: 10;
            padding-top: 0.75rem;
        }
        .tab-btn {
            background: none;
            border: none;
            border-bottom: 2px solid transparent;
            color: var(--text-muted);
            font-family: 'JetBrains Mono', monospace;
            font-size: 0.8rem;
            padding: 0.6rem 1.1rem;
            cursor: pointer;
            letter-spacing: 0.05em;
            text-transform: uppercase;
            transition: color 0.15s, border-color 0.15s;
            margin-bottom: -1px;
        }
        .tab-btn:hover { color: var(--text); }
        .tab-btn.active {
            color: var(--warlock);
            border-bottom-color: var(--warlock);
        }
        .tab-btn[data-tab="users"].active    { color: var(--sage);       border-bottom-color: var(--sage); }
        .tab-btn[data-tab="system"].active   { color: var(--accountant); border-bottom-color: var(--accountant); }
        .tab-btn[data-tab="lore"].active     { color: var(--chrono);     border-bottom-color: var(--chrono); }
        .tab-pane { display: none; }
        .tab-pane.active { display: block; }
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

    <nav class="tab-nav">
        <button class="tab-btn active" data-tab="access"  onclick="switchTab('access')">🔑 Access</button>
        <button class="tab-btn"        data-tab="users"   onclick="switchTab('users')">⚔️ Users</button>
        <button class="tab-btn"        data-tab="system"  onclick="switchTab('system')">🔧 System</button>
        <button class="tab-btn"        data-tab="lore"    onclick="switchTab('lore')">📜 Lore</button>
    </nav>

    <!-- ════════════════════════ TAB: ACCESS ════════════════════════ -->
    <div id="tab-access" class="tab-pane active">
    <!-- Web App Access — top priority panel -->
    <div class="panel">
        <div class="panel-title">Web App Access (webapp.questlinecli.com)</div>

        <!-- Generate code form -->
        <form method="POST" style="margin-bottom: 1.5rem;">
            <input type="hidden" name="action" value="generate_access_code">
            <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
            <div style="display:flex; gap:0.75rem; align-items:flex-end; margin-bottom:0.75rem;">
                <div style="flex:1;">
                    <label style="display:block; font-size:0.72rem; color:#666; margin-bottom:0.3rem; letter-spacing:0.1em; text-transform:uppercase;">Label (optional)</label>
                    <input type="text" name="label" placeholder="e.g. Ko-fi donation 2026-07-15" style="width:100%; background:#050505; border:1px solid #2a2a2a; border-radius:5px; color:#d4d4d4; font-family:inherit; font-size:0.85rem; padding:0.5rem 0.75rem;">
                </div>
                <button type="submit" class="btn" style="background:rgba(168,85,247,0.15); border-color:#a855f7; color:#a855f7; white-space:nowrap;">
                    Generate Code
                </button>
            </div>
            <div>
                <label style="display:block; font-size:0.72rem; color:#666; margin-bottom:0.3rem; letter-spacing:0.1em; text-transform:uppercase;">Link to CLI Account — Public Key (optional)</label>
                <input type="text" name="cli_public_key" placeholder="64-char hex public key from donor's CLI (questline identity show)" maxlength="64" style="width:100%; background:#050505; border:1px solid #2a2a2a; border-radius:5px; color:#06b6d4; font-family:inherit; font-size:0.8rem; padding:0.5rem 0.75rem; letter-spacing:0.05em;">
                <p style="font-size:0.72rem; color:#444; margin-top:0.3rem;">When set, the access code is pre-linked to the donor's existing CLI data. Their tasks and projects will appear when they log into the webapp.</p>
            </div>
        </form>

        <!-- Access codes table -->
        <table style="margin-bottom: 2rem;">
            <thead>
                <tr>
                    <th>Code</th>
                    <th>Label</th>
                    <th>Linked</th>
                    <th>Created</th>
                    <th>Status</th>
                    <th>Actions</th>
                </tr>
            </thead>
            <tbody>
                <?php if (empty($access_codes)): ?>
                    <tr><td colspan="6" style="text-align:center; color:#444;">No access codes yet.</td></tr>
                <?php else: ?>
                    <?php foreach ($access_codes as $ac): ?>
                        <?php
                        $redeemed = !empty($ac['redeemed_by_user_id']);
                        $revoked  = $ac['redeemed_by_user_id'] === 'REVOKED';
                        $statusLabel = $revoked ? 'Revoked' : ($redeemed ? 'Used' : 'Available');
                        $statusClass = $revoked ? 'badge-danger' : ($redeemed ? 'badge-info' : 'badge-success');
                        ?>
                        <tr>
                            <td><code style="letter-spacing:0.15em; color:#a855f7;"><?= htmlspecialchars($ac['code']) ?></code></td>
                            <td style="color:#888;"><?= htmlspecialchars($ac['label'] ?? '—') ?></td>
                            <td style="color:#06b6d4; font-size:0.78rem;">
                                <?= !empty($ac['linked_user_id']) ? '<span title="'.htmlspecialchars($ac['linked_user_id']).'">✓ linked</span>' : '<span style="color:#333;">—</span>' ?>
                            </td>
                            <td><?= date('Y-m-d', strtotime($ac['created_at'])) ?></td>
                            <td><span class="badge <?= $statusClass ?>"><?= $statusLabel ?></span></td>
                            <td style="display:flex; gap:0.4rem;">
                                <button class="btn btn-sm" data-key="<?= htmlspecialchars($ac['code']) ?>" onclick="copyKey(this)">Copy</button>
                                <?php if (!$redeemed): ?>
                                    <form method="POST" style="margin:0;" onsubmit="return confirm('Revoke this code?');">
                                        <input type="hidden" name="action" value="revoke_access_code">
                                        <input type="hidden" name="code" value="<?= htmlspecialchars($ac['code']) ?>">
                                        <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                                        <button type="submit" class="btn btn-sm btn-warning">Revoke</button>
                                    </form>
                                <?php endif; ?>
                            </td>
                        </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
            </tbody>
        </table>

        <!-- Supporters table -->
        <div style="font-size:0.72rem; color:#555; letter-spacing:0.15em; text-transform:uppercase; margin-bottom:0.75rem;">
            Current Supporters (<?= count($supporters) ?>)
        </div>
        <table>
            <thead>
                <tr>
                    <th>Username</th>
                    <th>Email</th>
                    <th>Public Key</th>
                    <th>Joined</th>
                    <th>Supporter</th>
                </tr>
            </thead>
            <tbody>
                <?php if (empty($supporters)): ?>
                    <tr><td colspan="5" style="text-align:center; color:#444;">No supporters yet.</td></tr>
                <?php else: ?>
                    <?php foreach ($supporters as $sup): ?>
                        <tr>
                            <td><?= htmlspecialchars($sup['username'] ?? '—') ?></td>
                            <td style="color:#888;"><?= htmlspecialchars($sup['email'] ?? '—') ?></td>
                            <td>
                                <button class="btn btn-sm" data-key="<?= htmlspecialchars($sup['public_key']) ?>" onclick="copyKey(this)">Copy Key</button>
                            </td>
                            <td><?= date('Y-m-d', strtotime($sup['created_at'])) ?></td>
                            <td>
                                <form method="POST" style="margin:0;">
                                    <input type="hidden" name="action" value="toggle_supporter">
                                    <input type="hidden" name="user_id" value="<?= htmlspecialchars($sup['id']) ?>">
                                    <input type="hidden" name="supporter" value="0">
                                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                                    <button type="submit" class="btn btn-sm btn-warning" onclick="return confirm('Revoke supporter access?')">Revoke</button>
                                </form>
                            </td>
                        </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
            </tbody>
        </table>
    </div>

    </div><!-- /tab-access -->

    <!-- ════════════════════════ TAB: USERS ════════════════════════ -->
    <div id="tab-users" class="tab-pane">
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

    </div><!-- /tab-users -->

    <!-- ════════════════════════ TAB: SYSTEM ════════════════════════ -->
    <div id="tab-system" class="tab-pane">
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
    </div><!-- /tab-system -->

    <!-- ════════════════════════ TAB: LORE ════════════════════════ -->
    <div id="tab-lore" class="tab-pane">
    <div class="panel" style="margin-top:0;">
        <div class="panel-title">📜 Lore Library Manager</div>

        <!-- Tab switcher -->
        <div style="display:flex;gap:.5rem;margin-bottom:1.5rem;">
            <button class="btn" id="tab-lore-btn"  onclick="switchLoreTab('lore')"  style="background:var(--warlock)">Lore Entries (<?= count($lore_entries) ?>)</button>
            <button class="btn" id="tab-quest-btn" onclick="switchLoreTab('quest')" style="background:var(--bg-card);border:1px solid var(--border)">Class Quests (<?= count($lore_quests) ?>)</button>
        </div>

        <!-- ── LORE ENTRIES TAB ─────────────────────────────────────────────── -->
        <div id="lore-entries">

            <?php if ($edit_lore_entry): ?>
            <!-- Edit form (shown when ?edit_lore=id) -->
            <div style="background:var(--bg-card);border:1px solid var(--warlock);border-radius:8px;padding:1.25rem;margin-bottom:1.5rem;">
                <div style="color:var(--warlock);font-weight:700;margin-bottom:1rem;">Edit Entry: <?= htmlspecialchars($edit_lore_entry['id']) ?></div>
                <form method="POST">
                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                    <input type="hidden" name="action" value="lore_edit">
                    <?= lore_entry_form_fields($edit_lore_entry) ?>
                    <div style="display:flex;gap:.5rem;margin-top:1rem;">
                        <button type="submit" class="btn" style="background:var(--warlock)">Save Changes</button>
                        <a href="<?= $_SERVER['PHP_SELF'] ?>" class="btn" style="background:var(--bg-card);border:1px solid var(--border)">Cancel</a>
                    </div>
                </form>
            </div>
            <?php else: ?>
            <!-- Add new entry form (collapsible) -->
            <details style="margin-bottom:1.5rem;">
                <summary class="btn" style="background:var(--bg-card);border:1px solid var(--warlock);cursor:pointer;display:inline-block;padding:.5rem 1rem;border-radius:6px;">+ Add Lore Entry</summary>
                <div style="background:var(--bg-card);border:1px solid var(--border);border-radius:8px;padding:1.25rem;margin-top:.75rem;">
                    <form method="POST">
                        <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                        <input type="hidden" name="action" value="lore_add">
                        <?= lore_entry_form_fields(null) ?>
                        <button type="submit" class="btn" style="background:var(--warlock);margin-top:1rem;">Add Entry</button>
                    </form>
                </div>
            </details>
            <?php endif; ?>

            <!-- Category filter -->
            <div style="display:flex;gap:.5rem;flex-wrap:wrap;margin-bottom:.5rem;" id="lore-cat-filters">
                <?php foreach (['All','World','Class','Memory','Achievement'] as $cat): ?>
                    <button class="btn <?= $cat==='All'?'active-cat':'' ?>"
                            style="padding:.3rem .75rem;font-size:.75rem;background:<?= $cat==='All'?'var(--warlock)':'var(--bg-card)' ?>;border:1px solid var(--border);"
                            onclick="filterLore('<?= $cat ?>')">
                        <?= $cat ?>
                    </button>
                <?php endforeach; ?>
            </div>
            <!-- Class sub-filter (only visible when Class category is active) -->
            <div id="class-subfilters" style="display:none;gap:.4rem;flex-wrap:wrap;margin-bottom:1rem;padding:.5rem;background:rgba(168,85,247,.07);border:1px solid rgba(168,85,247,.2);border-radius:6px;">
                <span style="font-size:.7rem;color:var(--text-dim);align-self:center;margin-right:.25rem;">Filter by class:</span>
                <?php
                $class_colors_map = [
                    'Code Warlock'      => 'var(--warlock)',
                    'Task Paladin'      => 'var(--paladin)',
                    'Mind Sage'         => 'var(--sage)',
                    'Systems Architect' => 'var(--architect)',
                    'Time Chronomancer' => 'var(--chrono)',
                    'Arch Accountant'   => 'var(--accountant)',
                ];
                ?>
                <button class="btn" style="padding:.25rem .6rem;font-size:.7rem;background:var(--bg-card);border:1px solid var(--border);" onclick="filterLoreClass('All')">All</button>
                <button class="btn" style="padding:.25rem .6rem;font-size:.7rem;background:var(--bg-card);border:1px solid var(--border);" onclick="filterLoreClass('shared')">Shared</button>
                <?php foreach ($class_colors_map as $cls => $col): ?>
                    <button class="btn" style="padding:.25rem .6rem;font-size:.7rem;background:var(--bg-card);border:1px solid <?= $col ?>;color:<?= $col ?>;" onclick="filterLoreClass(<?= json_encode($cls) ?>)">
                        <?= htmlspecialchars($cls) ?>
                    </button>
                <?php endforeach; ?>
            </div>

            <!-- Entries table -->
            <div style="overflow-x:auto;">
            <table id="lore-table">
                <thead>
                    <tr>
                        <th>ID</th>
                        <th>Cat</th>
                        <th>Class</th>
                        <th>Title</th>
                        <th>Unlock</th>
                        <th>Rarity</th>
                        <th>Sort</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                <?php if (empty($lore_entries)): ?>
                    <tr><td colspan="8" style="text-align:center;color:var(--text-muted)">No lore entries found. Check that server/data/lore.json exists.</td></tr>
                <?php else: ?>
                    <?php
                    $cls_colors_inline = [
                        'Code Warlock'      => 'var(--warlock)',
                        'Task Paladin'      => 'var(--paladin)',
                        'Mind Sage'         => 'var(--sage)',
                        'Systems Architect' => 'var(--architect)',
                        'Time Chronomancer' => 'var(--chrono)',
                        'Arch Accountant'   => 'var(--accountant)',
                    ];
                    ?>
                    <?php foreach ($lore_entries as $entry): ?>
                    <?php $entry_class = $entry['class_filter'] ?? null; ?>
                    <tr data-cat="<?= htmlspecialchars($entry['category']) ?>" data-class="<?= htmlspecialchars($entry_class ?? 'shared') ?>">
                        <td style="font-size:.7rem;color:var(--text-muted)"><?= htmlspecialchars($entry['id']) ?></td>
                        <td>
                            <span class="badge badge-info" style="font-size:.65rem;"><?= htmlspecialchars($entry['category']) ?></span>
                        </td>
                        <td style="font-size:.75rem;">
                            <?php if ($entry_class): ?>
                                <span style="color:<?= $cls_colors_inline[$entry_class] ?? 'var(--text-muted)' ?>;font-weight:600;"><?= htmlspecialchars($entry_class) ?></span>
                            <?php else: ?>
                                <span style="color:var(--text-muted)">—</span>
                            <?php endif; ?>
                        </td>
                        <td><?= htmlspecialchars($entry['title']) ?></td>
                        <td style="font-size:.75rem;color:var(--text-muted)">
                            <?= htmlspecialchars($entry['unlock']['type'] ?? '—') ?>
                            <?php if (!empty($entry['unlock']['level'])): ?>
                                <span style="color:var(--accountant)">lv<?= $entry['unlock']['level'] ?></span>
                            <?php endif; ?>
                        </td>
                        <td>
                            <?php
                            $r = $entry['rarity'] ?? null;
                            $rc = $r === 'legendary' ? 'var(--accountant)' : ($r === 'rare' ? 'var(--sage)' : 'var(--text-muted)');
                            ?>
                            <?php if ($r): ?>
                                <span style="color:<?= $rc ?>;font-size:.75rem;"><?= htmlspecialchars($r) ?></span>
                            <?php else: ?>
                                <span style="color:var(--text-muted);font-size:.75rem;">—</span>
                            <?php endif; ?>
                        </td>
                        <td style="font-size:.75rem;"><?= $entry['sort_order'] ?? 0 ?></td>
                        <td>
                            <div style="display:flex;gap:.4rem;">
                                <a href="<?= $_SERVER['PHP_SELF'] ?>?edit_lore=<?= urlencode($entry['id']) ?>#tab-lore-btn"
                                   class="btn" style="padding:.3rem .6rem;font-size:.7rem;background:var(--bg-card);border:1px solid var(--border);">Edit</a>
                                <form method="POST" onsubmit="return confirm('Delete \'<?= htmlspecialchars(addslashes($entry['id'])) ?>\'?')">
                                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                                    <input type="hidden" name="action"   value="lore_delete">
                                    <input type="hidden" name="entry_id" value="<?= htmlspecialchars($entry['id']) ?>">
                                    <button type="submit" class="btn" style="padding:.3rem .6rem;font-size:.7rem;background:var(--danger);">Del</button>
                                </form>
                            </div>
                        </td>
                    </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
                </tbody>
            </table>
            </div>
        </div><!-- /lore-entries -->

        <!-- ── CLASS QUESTS TAB ─────────────────────────────────────────────── -->
        <div id="lore-quests" style="display:none;">

            <?php if ($edit_quest_entry): ?>
            <div style="background:var(--bg-card);border:1px solid var(--accountant);border-radius:8px;padding:1.25rem;margin-bottom:1.5rem;">
                <div style="color:var(--accountant);font-weight:700;margin-bottom:1rem;">
                    Edit Quest: <?= htmlspecialchars($edit_quest_entry['class']) ?> lv<?= $edit_quest_entry['level'] ?>
                </div>
                <form method="POST">
                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                    <input type="hidden" name="action"      value="quest_edit">
                    <input type="hidden" name="orig_class"  value="<?= htmlspecialchars($edit_quest_entry['class']) ?>">
                    <input type="hidden" name="orig_level"  value="<?= $edit_quest_entry['level'] ?>">
                    <?= quest_form_fields($edit_quest_entry) ?>
                    <div style="display:flex;gap:.5rem;margin-top:1rem;">
                        <button type="submit" class="btn" style="background:var(--accountant)">Save Changes</button>
                        <a href="<?= $_SERVER['PHP_SELF'] ?>" class="btn" style="background:var(--bg-card);border:1px solid var(--border)">Cancel</a>
                    </div>
                </form>
            </div>
            <?php else: ?>
            <details style="margin-bottom:1.5rem;">
                <summary class="btn" style="background:var(--bg-card);border:1px solid var(--accountant);cursor:pointer;display:inline-block;padding:.5rem 1rem;border-radius:6px;">+ Add Class Quest</summary>
                <div style="background:var(--bg-card);border:1px solid var(--border);border-radius:8px;padding:1.25rem;margin-top:.75rem;">
                    <form method="POST">
                        <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                        <input type="hidden" name="action" value="quest_add">
                        <?= quest_form_fields(null) ?>
                        <button type="submit" class="btn" style="background:var(--accountant);margin-top:1rem;">Add Quest</button>
                    </form>
                </div>
            </details>
            <?php endif; ?>

            <!-- Class filter -->
            <?php
            $all_classes = array_unique(array_column($lore_quests, 'class'));
            sort($all_classes);
            ?>
            <div style="display:flex;gap:.5rem;flex-wrap:wrap;margin-bottom:1rem;" id="quest-class-filters">
                <button class="btn" style="padding:.3rem .75rem;font-size:.75rem;background:var(--accountant);border:1px solid var(--border);" onclick="filterQuests('All')">All</button>
                <?php foreach ($all_classes as $cls): ?>
                    <button class="btn" style="padding:.3rem .75rem;font-size:.75rem;background:var(--bg-card);border:1px solid var(--border);" onclick="filterQuests('<?= htmlspecialchars($cls) ?>')">
                        <?= htmlspecialchars($cls) ?>
                    </button>
                <?php endforeach; ?>
            </div>

            <div style="overflow-x:auto;">
            <table id="quest-table">
                <thead>
                    <tr>
                        <th>Class</th>
                        <th>Lv</th>
                        <th>Name</th>
                        <th>Objective</th>
                        <th>Reward ID</th>
                        <th>Actions</th>
                    </tr>
                </thead>
                <tbody>
                <?php if (empty($lore_quests)): ?>
                    <tr><td colspan="6" style="text-align:center;color:var(--text-muted)">No quests found. Check server/data/quests.json.</td></tr>
                <?php else: ?>
                    <?php foreach ($lore_quests as $q): ?>
                    <tr data-class="<?= htmlspecialchars($q['class']) ?>">
                        <td>
                            <?php
                            $cls_colors = [
                                'Code Warlock'      => 'var(--warlock)',
                                'Task Paladin'      => 'var(--paladin)',
                                'Mind Sage'         => 'var(--sage)',
                                'Systems Architect' => 'var(--architect)',
                                'Time Chronomancer' => 'var(--chrono)',
                                'Arch Accountant'   => 'var(--accountant)',
                            ];
                            $cc = $cls_colors[$q['class']] ?? 'var(--text-muted)';
                            ?>
                            <span style="color:<?= $cc ?>;font-size:.75rem;"><?= htmlspecialchars($q['class']) ?></span>
                        </td>
                        <td style="font-size:.85rem;"><?= $q['level'] ?></td>
                        <td><?= htmlspecialchars($q['name']) ?></td>
                        <td style="font-size:.75rem;color:var(--text-muted);">
                            <?= htmlspecialchars($q['objective']['type'] ?? '—') ?>
                            × <?= $q['objective']['target'] ?? 0 ?>
                        </td>
                        <td style="font-size:.7rem;color:var(--text-muted);"><?= htmlspecialchars($q['reward_lore_id'] ?? '—') ?></td>
                        <td>
                            <div style="display:flex;gap:.4rem;">
                                <a href="<?= $_SERVER['PHP_SELF'] ?>?edit_quest=<?= urlencode($q['class'].'|'.$q['level']) ?>#tab-quest-btn"
                                   class="btn" style="padding:.3rem .6rem;font-size:.7rem;background:var(--bg-card);border:1px solid var(--border);">Edit</a>
                                <form method="POST" onsubmit="return confirm('Delete this quest?')">
                                    <input type="hidden" name="csrf_token" value="<?= htmlspecialchars($csrf_token) ?>">
                                    <input type="hidden" name="action"      value="quest_delete">
                                    <input type="hidden" name="quest_class" value="<?= htmlspecialchars($q['class']) ?>">
                                    <input type="hidden" name="quest_level" value="<?= $q['level'] ?>">
                                    <button type="submit" class="btn" style="padding:.3rem .6rem;font-size:.7rem;background:var(--danger);">Del</button>
                                </form>
                            </div>
                        </td>
                    </tr>
                    <?php endforeach; ?>
                <?php endif; ?>
                </tbody>
            </table>
            </div>
        </div><!-- /tab-quest -->
    </div><!-- /panel Lore Library -->

    </div><!-- /tab-lore -->

    <script>
        // ── Tab switching — persiste en el hash de la URL ─────────────────────
        function switchTab(name) {
            document.querySelectorAll('.tab-pane').forEach(p => p.classList.remove('active'));
            document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
            document.getElementById('tab-' + name).classList.add('active');
            document.querySelector('.tab-btn[data-tab="' + name + '"]').classList.add('active');
            history.replaceState(null, '', '#' + name);
        }

        // Restaura el tab activo desde el hash al cargar
        (function () {
            const tab = location.hash.replace('#', '') || 'access';
            const valid = ['access', 'users', 'system', 'lore'];
            switchTab(valid.includes(tab) ? tab : 'access');
        })();

        function copyKey(btn) {
            const key = btn.dataset.key;
            const original = btn.textContent;

            function onSuccess() {
                btn.textContent = 'Copied!';
                setTimeout(() => { btn.textContent = original; }, 1800);
            }

            function fallback() {
                // execCommand fallback for HTTP environments
                const ta = document.createElement('textarea');
                ta.value = key;
                ta.style.cssText = 'position:fixed;top:-9999px;left:-9999px;opacity:0';
                document.body.appendChild(ta);
                ta.select();
                const ok = document.execCommand('copy');
                document.body.removeChild(ta);
                if (ok) { onSuccess(); } else { prompt('Copy this code:', key); }
            }

            if (navigator.clipboard && window.isSecureContext) {
                navigator.clipboard.writeText(key).then(onSuccess).catch(fallback);
            } else {
                fallback();
            }
        }

        // ── Lore Library tab switching ────────────────────────────────────────
        function switchLoreTab(tab) {
            const isLore = tab === 'lore';
            document.getElementById('lore-entries').style.display = isLore ? '' : 'none';
            document.getElementById('lore-quests').style.display  = isLore ? 'none' : '';
            document.getElementById('tab-lore-btn').style.background  = isLore ? 'var(--warlock)' : 'var(--bg-card)';
            document.getElementById('tab-quest-btn').style.background = isLore ? 'var(--bg-card)' : 'var(--accountant)';
        }

        let _currentLoreCat = 'All';
        let _currentLoreClass = 'All';

        function filterLore(cat) {
            _currentLoreCat = cat;
            _currentLoreClass = 'All';
            applyLoreFilters();
            // Muestra el sub-filtro de clase solo cuando está activo el filtro "Class"
            const subFilters = document.getElementById('class-subfilters');
            if (subFilters) subFilters.style.display = (cat === 'Class' || cat === 'All') ? 'flex' : 'none';
            document.querySelectorAll('#lore-cat-filters .btn').forEach(b => {
                b.style.background = (b.textContent.trim() === cat) ? 'var(--warlock)' : 'var(--bg-card)';
            });
        }

        function filterLoreClass(cls) {
            _currentLoreClass = cls;
            applyLoreFilters();
            document.querySelectorAll('#class-subfilters .btn').forEach(b => {
                const active = b.textContent.trim() === cls || (cls === 'All' && b.textContent.trim() === 'All');
                b.style.fontWeight = active ? '700' : '';
                b.style.borderWidth = active ? '2px' : '';
            });
        }

        function applyLoreFilters() {
            document.querySelectorAll('#lore-table tbody tr').forEach(tr => {
                const catMatch = (_currentLoreCat === 'All' || tr.dataset.cat === _currentLoreCat);
                let classMatch = true;
                if (_currentLoreClass !== 'All' && tr.dataset.cat === 'Class') {
                    if (_currentLoreClass === 'shared') {
                        classMatch = (tr.dataset.class === 'shared');
                    } else {
                        classMatch = (tr.dataset.class === _currentLoreClass);
                    }
                }
                tr.style.display = (catMatch && classMatch) ? '' : 'none';
            });
        }

        function filterQuests(cls) {
            document.querySelectorAll('#quest-table tbody tr').forEach(tr => {
                tr.style.display = (cls === 'All' || tr.dataset.class === cls) ? '' : 'none';
            });
            document.querySelectorAll('#quest-class-filters .btn').forEach(b => {
                const active = b.textContent.trim() === cls;
                b.style.background = active ? 'var(--accountant)' : 'var(--bg-card)';
            });
        }

        // Conditional fields for unlock type in lore form
        function updateUnlockFields(sel) {
            const v = sel.value;
            const show = (id, cond) => {
                const el = document.getElementById(id);
                if (el) el.closest('.lore-field-row').style.display = cond ? '' : 'none';
            };
            show('unlock_level',       v === 'level' || v === 'class_level');
            show('unlock_class',       v === 'class_level');
            show('unlock_milestone_id',v === 'milestone');
            show('unlock_chapter_id',  v === 'chapter_reward');
        }

        // Auto-switch to quest tab if URL has edit_quest
        const _qs = new URLSearchParams(location.search);
        if (_qs.has('edit_lore') || _qs.has('edit_quest')) switchTab('lore');
        if (_qs.has('edit_quest')) switchLoreTab('quest');
    </script>
</body>
</html>
