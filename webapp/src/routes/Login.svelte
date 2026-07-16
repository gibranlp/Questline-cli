<script>
  import { saveIdentity, storeKeyInSession } from '../lib/auth.js';
  import { loginWebapp } from '../lib/api.js';
  import { decryptSecretKey, deriveDataKey, exportKeyHex } from '../lib/crypto.js';
  import { identity as identityStore, dataKey as dataKeyStore } from '../lib/store.js';
  import { navigate } from '../lib/router.js';

  const secureContext = window.isSecureContext && crypto?.subtle != null;

  let username = '';
  let password = '';
  let errorMsg = '';
  let loading = false;

  async function login() {
    errorMsg = '';
    if (!username.trim() || !password) return;
    loading = true;

    try {
      const data = await loginWebapp(username.trim(), password);
      const secretKey = await decryptSecretKey(data.encrypted_key_blob, password);

      const identity = {
        user_uuid:  data.user_id,
        public_key: data.public_key,
        secret_key: secretKey,
        device_id:  crypto.randomUUID(),
        created_at: new Date().toISOString(),
      };

      // Derive the data encryption key and store its bytes in sessionStorage
      const key = await deriveDataKey(password, data.user_id);
      const keyHex = await exportKeyHex(key);
      storeKeyInSession(keyHex);
      dataKeyStore.set(key);

      saveIdentity(identity);
      identityStore.set(identity);
      navigate('/dashboard');
    } catch (err) {
      errorMsg = err.message || 'Login failed';
      loading = false;
    }
  }
</script>

<div class="login-page">
  <div class="logo-wrap">
    <img class="logo-img" src="/assets/logo.png" alt="QUESTLINE" />
  </div>

  <div class="login-card">
    {#if !secureContext}
      <div class="no-https">
        <strong>HTTPS required</strong>
        <p>Visit <code>https://webapp.questlinecli.com</code></p>
      </div>

    {:else if loading}
      <div class="loading">
        <div class="spinner"></div>
        <p>Authenticating…</p>
      </div>

    {:else}
      <form on:submit|preventDefault={login} class="form">
        <div class="field">
          <label for="username">Username</label>
          <input
            id="username"
            type="text"
            bind:value={username}
            placeholder="your_username"
            autocomplete="username"
            spellcheck="false"
          />
        </div>

        <div class="field">
          <label for="password">Password</label>
          <input
            id="password"
            type="password"
            bind:value={password}
            placeholder="••••••••"
            autocomplete="current-password"
          />
        </div>

        {#if errorMsg}
          <div class="error">{errorMsg}</div>
        {/if}

        <button type="submit" class="btn-primary" disabled={!username.trim() || !password}>
          Enter the Realm
        </button>

        <p class="hint">
          New supporter?
          <a href="/register" on:click|preventDefault={() => navigate('/register')}>Register with access code →</a>
        </p>
      </form>
    {/if}
  </div>
</div>

<style>
  .login-page {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    gap: 2rem;
    font-family: 'JetBrains Mono', monospace;
    position: relative;
    background:
      linear-gradient(to bottom, rgba(8,8,8,0.72) 0%, rgba(8,8,8,0.48) 50%, rgba(8,8,8,0.78) 100%),
      url('/assets/home.webp') center / cover no-repeat fixed;
  }

  /* scanline overlay — matches server/styles.css */
  .login-page::before {
    content: '';
    pointer-events: none;
    position: fixed;
    inset: 0;
    background: repeating-linear-gradient(
      0deg, transparent, transparent 2px,
      rgba(0,0,0,0.06) 2px, rgba(0,0,0,0.06) 4px
    );
    z-index: 0;
  }

  /* ── Logo — outside the card, large, matches index.html #logo-img ── */
  .logo-wrap {
    position: relative;
    z-index: 1;
    width: 100%;
    overflow-x: auto;
  }

  .logo-img {
    max-width: min(780px, 90vw);
    width: 100%;
    height: auto;
    display: block;
    margin: 0 auto;
    animation: logoGlow 9s linear infinite;
  }

  /* Exact match of index.html logoGlow */
  @keyframes logoGlow {
    0%   { filter: drop-shadow(0 0 4px rgba(168,  85, 247, 0.3)); }
    16%  { filter: drop-shadow(0 0 4px rgba(255, 105, 180, 0.3)); }
    33%  { filter: drop-shadow(0 0 4px rgba(  6, 182, 212, 0.3)); }
    50%  { filter: drop-shadow(0 0 4px rgba( 59, 130, 246, 0.3)); }
    66%  { filter: drop-shadow(0 0 4px rgba(249, 115,  22, 0.3)); }
    83%  { filter: drop-shadow(0 0 4px rgba(245, 158,  11, 0.3)); }
    100% { filter: drop-shadow(0 0 4px rgba(168,  85, 247, 0.3)); }
  }

  .login-card {
    position: relative;
    z-index: 1;
    width: 100%;
    max-width: 400px;
    background: rgba(8, 8, 8, 0.88);
    border: none;
    border-radius: 14px;
    padding: 2rem 2.5rem;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    text-align: center;
    box-shadow: 0 0 60px rgba(0,0,0,0.6);
  }

  .login-card::after {
    content: '';
    position: absolute;
    inset: -44px;
    background: url('/assets/ui/bfg.png') center / 100% 100% no-repeat;
    pointer-events: none;
    z-index: 10;
  }

  .form { text-align: left; }
  .field { margin-bottom: 1.25rem; }

  label {
    display: block;
    font-size: 0.72rem;
    color: #555;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    margin-bottom: 0.45rem;
  }

  input {
    width: 100%;
    background: rgba(0,0,0,0.5);
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.9rem;
    padding: 0.65rem 0.9rem;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  input:focus {
    border-color: #a855f7;
    box-shadow: 0 0 0 3px rgba(168,85,247,0.1);
  }

  .error {
    background: rgba(239,68,68,0.1);
    border: 1px solid #ef4444;
    border-radius: 6px;
    padding: 0.6rem 0.9rem;
    color: #f87171;
    font-size: 0.82rem;
    margin-bottom: 1rem;
  }

  .btn-primary {
    width: 100%;
    background: rgba(168,85,247,0.15);
    border: 1px solid #a855f7;
    border-radius: 6px;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    letter-spacing: 0.12em;
    padding: 0.75rem;
    cursor: pointer;
    text-transform: uppercase;
    transition: background 0.15s, color 0.15s, box-shadow 0.15s;
  }

  .btn-primary:hover:not(:disabled) {
    background: rgba(168,85,247,0.28);
    color: #c084fc;
    box-shadow: 0 0 16px rgba(168,85,247,0.2);
  }

  .btn-primary:disabled { opacity: 0.35; cursor: not-allowed; }

  .hint {
    text-align: center;
    font-size: 0.75rem;
    color: #444;
    margin-top: 1.25rem;
    line-height: 1.6;
  }

  .hint a { color: #06b6d4; text-decoration: none; }
  .hint a:hover { text-decoration: underline; }

  .loading { padding: 1rem 0; color: #888; text-align: center; }

  .spinner {
    width: 32px;
    height: 32px;
    border: 2px solid #1c1c1c;
    border-top-color: #a855f7;
    border-radius: 50%;
    margin: 0 auto 1rem;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .no-https {
    padding: 1.5rem;
    background: rgba(239,68,68,0.08);
    border: 1px solid #ef4444;
    border-radius: 8px;
    color: #f87171;
  }

  .no-https strong { display: block; font-size: 0.9rem; margin-bottom: 0.5rem; color: #fca5a5; }
  .no-https p { font-size: 0.8rem; color: #888; }
  .no-https code { color: #06b6d4; font-family: inherit; }
</style>
