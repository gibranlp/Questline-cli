<?php
// ─────────────────────────────────────────────────────────────────────────────
// load_env.php — carga las variables de entorno del archivo .env
// ─────────────────────────────────────────────────────────────────────────────
/**
 * Questline Environment Loader
 * Loads key-value pairs from .env files into environment variables.
 */

function load_env() {
    // Look for a .env file in the current directory, the server directory, or the root directory.
    $paths = [
        __DIR__ . '/.env',
        dirname(__DIR__) . '/.env',
        __DIR__ . '/api/.env',
        __DIR__ . '/admin/.env'
    ];
    
    $envPath = null;
    foreach ($paths as $path) {
        if (file_exists($path)) {
            $envPath = $path;
            break;
        }
    }
    
    if (!$envPath) {
        return; // No .env file found; fallback to existing environment variables
    }
    
    $lines = @file($envPath, FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES);
    if ($lines === false) {
        return;
    }
    
    foreach ($lines as $line) {
        $line = trim($line);
        // Skip comments and empty lines
        if (empty($line) || $line[0] === '#') {
            continue;
        }
        
        // Parse key=value
        if (strpos($line, '=') !== false) {
            list($key, $value) = explode('=', $line, 2);
            $key = trim($key);
            $value = trim($value);
            
            // Strip quotes if present
            $len = strlen($value);
            if ($len >= 2 && (
                ($value[0] === '"' && $value[$len - 1] === '"') ||
                ($value[0] === "'" && $value[$len - 1] === "'")
            )) {
                $value = substr($value, 1, -1);
            }
            
            // Populate environment variables
            putenv("$key=$value");
            $_ENV[$key] = $value;
            $_SERVER[$key] = $value;
        }
    }
}

// Automatically execute on include
load_env();
