<script>
  import { saveIdentity, storeKeyInSession } from '../lib/auth.js';
  import { loginWebapp } from '../lib/api.js';
  import { decryptSecretKey, deriveDataKey, exportKeyHex } from '../lib/crypto.js';
  import { identity as identityStore, dataKey as dataKeyStore } from '../lib/store.js';
  import { navigate } from '../lib/router.js';

  const secureContext = window.isSecureContext && crypto?.subtle != null;

  let imagesReady = false;

  const UI_IMAGES = [
    '/assets/ui/gc-n.webp', '/assets/ui/gc-h.webp', '/assets/ui/gc-p.webp',
    '/assets/ui/i-n.webp',  '/assets/ui/i-a.webp',  '/assets/ui/i-e.webp',
    '/assets/ui/gf-tlc.webp', '/assets/ui/gf-trc.webp',
    '/assets/ui/gf-blc.webp', '/assets/ui/gf-brc.webp',
    '/assets/ui/gf-et.webp',  '/assets/ui/gf-eb.webp',
    '/assets/ui/gf-el.webp',  '/assets/ui/gf-er.webp',
    '/assets/ui/gf-gemt.webp', '/assets/ui/gf-gemb.webp',
  ];

  Promise.all(
    UI_IMAGES.map(src => new Promise(resolve => {
      const img = new Image();
      img.onload = img.onerror = resolve;
      img.src = src;
    }))
  ).then(() => { imagesReady = true; });

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
    <div class="gf-border" aria-hidden="true">
      <div class="gf-corner gf-tlc"></div>
      <div class="gf-corner gf-trc"></div>
      <div class="gf-corner gf-blc"></div>
      <div class="gf-corner gf-brc"></div>
      <div class="gf-edge gf-et"></div>
      <div class="gf-edge gf-eb"></div>
      <div class="gf-edge gf-el"></div>
      <div class="gf-edge gf-er"></div>
      <div class="gf-gem gf-gemt"></div>
      <div class="gf-gem gf-gemb"></div>
    </div>
    {#if !secureContext}
      <div class="no-https">
        <strong>HTTPS required</strong>
        <p>Visit <code>https://webapp.questlinecli.com</code></p>
      </div>

    {:else if !imagesReady}
      <div class="loading">
        <div class="spinner"></div>
      </div>

    {:else if loading}
      <div class="loading">
        <div class="spinner"></div>
        <p>Authenticating…</p>
      </div>

    {:else}
      <form on:submit|preventDefault={login} class="form">
        <div class="field" class:err={!!errorMsg}>
          <label for="username">Username</label>
          <input
            id="username"
            type="text"
            bind:value={username}
            class:err={!!errorMsg}
            on:input={() => { if (errorMsg) errorMsg = ''; }}
            placeholder="your_username"
            autocomplete="username"
            spellcheck="false"
          />
        </div>

        <div class="field" class:err={!!errorMsg}>
          <label for="password">Password</label>
          <input
            id="password"
            type="password"
            bind:value={password}
            class:err={!!errorMsg}
            on:input={() => { if (errorMsg) errorMsg = ''; }}
            placeholder="••••••••"
            autocomplete="current-password"
          />
        </div>

        {#if errorMsg}
          <div class="error">{errorMsg}</div>
        {/if}

        <button type="submit" class="btn-primary" aria-label="Enter the Realm"></button>

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
    font-family: inherit;
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
    border-radius: 0;
    padding: 2rem 2.5rem;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    text-align: center;
    box-shadow: 0 0 60px rgba(0,0,0,0.6);
  }

  /* ── Golden Frame Border ── */
  .gf-border {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 10;
    overflow: visible;
  }

  /* corners: flush with card corners, overlap inward so no gap with background */
  .gf-corner {
    position: absolute;
    width: 32px;
    height: 42px;
    background-size: 100% 100%;
    background-repeat: no-repeat;
  }
  .gf-tlc { top: 0; left: 0;    background-image: url('/assets/ui/gf-tlc.webp'); }
  .gf-trc { top: 0; right: 0;   background-image: url('/assets/ui/gf-trc.webp'); }
  .gf-blc { bottom: 0; left: 0;  background-image: url('/assets/ui/gf-blc.webp'); }
  .gf-brc { bottom: 0; right: 0; background-image: url('/assets/ui/gf-brc.webp'); }

  /* top/bottom edges: 3px, run between corners along the card boundary */
  .gf-et, .gf-eb {
    position: absolute;
    left: 32px; right: 32px;
    height: 3px;
    background-size: auto 100%;
    background-repeat: repeat-x;
  }
  .gf-et { top: 0;    background-image: url('/assets/ui/gf-et.webp'); }
  .gf-eb { bottom: 0; background-image: url('/assets/ui/gf-eb.webp'); }

  /* left/right edges: 3px, run between corners along the card boundary */
  .gf-el, .gf-er {
    position: absolute;
    top: 42px; bottom: 42px;
    width: 3px;
    background-size: 100% auto;
    background-repeat: repeat-y;
  }
  .gf-el { left: 0;  background-image: url('/assets/ui/gf-el.webp'); }
  .gf-er { right: 0; background-image: url('/assets/ui/gf-er.webp'); }

  /* gems: centered on the top/bottom edge line (top:0 / bottom:0) */
  .gf-gem {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    background-size: 100% 100%;
    background-repeat: no-repeat;
  }
  .gf-gemt {
    top: -21px;
    width: 86px;
    height: 45px;
    background-image: url('/assets/ui/gf-gemt.webp');
  }
  .gf-gemb {
    bottom: -11px;
    width: 58px;
    height: 25px;
    background-image: url('/assets/ui/gf-gemb.webp');
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
    background: url('/assets/ui/i-n.webp') center / 100% 100% no-repeat;
    border: none;
    border-radius: 0;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.9rem;
    padding: 0.65rem 0.9rem;
    outline: none;
  }

  input:focus {
    background-image: url('/assets/ui/i-a.webp');
  }

  input.err,
  input.err:focus {
    background-image: url('/assets/ui/i-e.webp');
  }

  .field.err label {
    color: #ef4444;
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
    display: block;
    width: 158px;
    height: 63px;
    margin: 0 auto;
    background: url('/assets/ui/gc-n.webp') center / 100% 100% no-repeat;
    border: none;
    border-radius: 0;
    padding: 0;
    cursor: pointer;
  }

  .btn-primary:hover {
    background-image: url('/assets/ui/gc-h.webp');
  }

  .btn-primary:active {
    background-image: url('/assets/ui/gc-p.webp');
  }

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
