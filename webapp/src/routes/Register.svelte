<script>
  import { generateIdentity, saveIdentity } from '../lib/auth.js';
  import { registerWithCredentials, checkAccessCode } from '../lib/api.js';
  import { encryptSecretKey } from '../lib/crypto.js';
  import { identity as identityStore } from '../lib/store.js';
  import { navigate } from '../lib/router.js';

  const secureContext = window.isSecureContext && crypto?.subtle != null;

  let imagesReady = false;

  const UI_IMAGES = [
    '/assets/ui/gc-n.webp', '/assets/ui/gc-h.webp', '/assets/ui/gc-p.webp',
    '/assets/ui/i-n.webp', '/assets/ui/i-a.webp', '/assets/ui/i-e.webp',
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

  let step = 'code'; // code | credentials | generating | done
  let accessCode = '';
  let username = '';
  let password = '';
  let passwordConfirm = '';
  let errorMsg = '';
  let checkingCode = false;

  async function goToCredentials() {
    errorMsg = '';
    if (!accessCode.trim()) return;
    checkingCode = true;
    try {
      const result = await checkAccessCode(accessCode.trim());
      if (!result.valid) {
        errorMsg = 'Invalid access code';
        return;
      }
      step = 'credentials';
    } catch {
      errorMsg = 'Invalid access code';
    } finally {
      checkingCode = false;
    }
  }

  async function register() {
    errorMsg = '';
    if (!username.trim() || !password) return;
    if (password !== passwordConfirm) {
      errorMsg = 'Passwords do not match';
      return;
    }
    if (password.length < 8) {
      errorMsg = 'Password must be at least 8 characters';
      return;
    }
    step = 'generating';

    try {
      const id = await generateIdentity();
      const encryptedKeyBlob = await encryptSecretKey(id.secret_key, password);

      const result = await registerWithCredentials({
        accessCode:      accessCode.trim(),
        username:        username.trim(),
        password:        password,
        identity:        id,
        encryptedKeyBlob,
      });

      // Server may return a different user_id if the code was linked to an existing CLI account
      if (result.user_id && result.user_id !== id.user_uuid) {
        id.user_uuid = result.user_id;
      }

      saveIdentity(id);
      identityStore.set(id);
      step = 'done';
      setTimeout(() => navigate('/dashboard'), 1500);
    } catch (err) {
      step = 'credentials';
      errorMsg = err.message || 'Registration failed';
    }
  }
</script>

<div class="register-page">
  <div class="logo-wrap">
    <img class="logo-img" src="/assets/logo.png" alt="QUESTLINE" />
  </div>

  <div class="register-card">
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
        <p>This app requires a secure connection. Visit <code>https://webapp.questlinecli.com</code></p>
      </div>

    {:else if !imagesReady}
      <div class="loading">
        <div class="spinner"></div>
      </div>

    {:else if step === 'code'}
      <form on:submit|preventDefault={goToCredentials} class="form">
        <div class="field" class:err={!!errorMsg}>
          <label for="code">Access Code</label>
          <input
            id="code"
            type="text"
            bind:value={accessCode}
            class:err={!!errorMsg}
            on:input={() => { if (errorMsg) errorMsg = ''; }}
            placeholder="XXXX-XXXX-XXXX-XXXX"
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        {#if errorMsg}
          <div class="error">{errorMsg}</div>
        {/if}
        <button type="submit" class="btn-primary" disabled={!accessCode.trim() || checkingCode} aria-label="Continue"></button>
        <p class="hint">
          Access codes are issued to supporters via Ko-fi.
          <a href="https://ko-fi.com/questlinecli" target="_blank" rel="noopener">
            Support the project →
          </a>
        </p>
        <p class="hint" style="margin-top:0.75rem;">
          Already have an account?
          <a href="/login" on:click|preventDefault={() => navigate('/login')}>Log in instead →</a>
        </p>
      </form>

    {:else if step === 'credentials'}
      <form on:submit|preventDefault={register} class="form">
        <p class="step-note">Code accepted. Set your login credentials.</p>

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
            maxlength="50"
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
            placeholder="at least 8 characters"
            autocomplete="new-password"
          />
        </div>

        <div class="field" class:err={!!errorMsg}>
          <label for="passwordConfirm">Confirm Password</label>
          <input
            id="passwordConfirm"
            type="password"
            bind:value={passwordConfirm}
            class:err={!!errorMsg}
            on:input={() => { if (errorMsg) errorMsg = ''; }}
            placeholder="repeat password"
            autocomplete="new-password"
          />
        </div>

        {#if errorMsg}
          <div class="error">{errorMsg}</div>
        {/if}

        <button type="submit" class="btn-primary" disabled={!username.trim() || !password || !passwordConfirm} aria-label="Forge Identity"></button>

        <p class="hint" style="margin-top:0.75rem;">
          <a href="#back" on:click|preventDefault={() => { step = 'code'; errorMsg = ''; }}>← Back</a>
        </p>
      </form>

    {:else if step === 'generating'}
      <div class="loading">
        <div class="spinner"></div>
        <p>Forging your identity…</p>
      </div>

    {:else if step === 'done'}
      <div class="success">
        <div class="success-icon">⚔️</div>
        <p>Welcome to the Realm.</p>
        <p class="dim">Redirecting to dashboard…</p>
      </div>
    {/if}

  </div><!-- closes register-card -->
</div>

<style>
  .register-page {
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

  .register-page::before {
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

  @keyframes logoGlow {
    0%   { filter: drop-shadow(0 0 4px rgba(168,  85, 247, 0.3)); }
    16%  { filter: drop-shadow(0 0 4px rgba(255, 105, 180, 0.3)); }
    33%  { filter: drop-shadow(0 0 4px rgba(  6, 182, 212, 0.3)); }
    50%  { filter: drop-shadow(0 0 4px rgba( 59, 130, 246, 0.3)); }
    66%  { filter: drop-shadow(0 0 4px rgba(249, 115,  22, 0.3)); }
    83%  { filter: drop-shadow(0 0 4px rgba(245, 158,  11, 0.3)); }
    100% { filter: drop-shadow(0 0 4px rgba(168,  85, 247, 0.3)); }
  }

  .register-card {
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

  .gf-corner {
    position: absolute;
    width: 32px;
    height: 42px;
    background-size: 100% 100%;
    background-repeat: no-repeat;
  }
  .gf-tlc { top: 0; left: 0;     background-image: url('/assets/ui/gf-tlc.webp'); }
  .gf-trc { top: 0; right: 0;    background-image: url('/assets/ui/gf-trc.webp'); }
  .gf-blc { bottom: 0; left: 0;  background-image: url('/assets/ui/gf-blc.webp'); }
  .gf-brc { bottom: 0; right: 0; background-image: url('/assets/ui/gf-brc.webp'); }

  .gf-et, .gf-eb {
    position: absolute;
    left: 32px; right: 32px;
    height: 3px;
    background-size: auto 100%;
    background-repeat: repeat-x;
  }
  .gf-et { top: 0;    background-image: url('/assets/ui/gf-et.webp'); }
  .gf-eb { bottom: 0; background-image: url('/assets/ui/gf-eb.webp'); }

  .gf-el, .gf-er {
    position: absolute;
    top: 42px; bottom: 42px;
    width: 3px;
    background-size: 100% auto;
    background-repeat: repeat-y;
  }
  .gf-el { left: 0;  background-image: url('/assets/ui/gf-el.webp'); }
  .gf-er { right: 0; background-image: url('/assets/ui/gf-er.webp'); }

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

  .step-note {
    font-size: 0.8rem;
    color: #666;
    margin-bottom: 1.25rem;
    text-align: left;
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

  .btn-primary:hover:not(:disabled) {
    background-image: url('/assets/ui/gc-h.webp');
  }

  .btn-primary:active:not(:disabled) {
    background-image: url('/assets/ui/gc-p.webp');
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

  .loading, .success {
    padding: 1rem 0;
    color: #888;
    text-align: center;
  }

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

  .success-icon { font-size: 2rem; margin-bottom: 0.75rem; }
  .success p { color: #d4d4d4; font-size: 0.9rem; }

  .dim { color: #555 !important; font-size: 0.8rem !important; margin-top: 0.25rem; }

  .no-https {
    padding: 1.5rem;
    background: rgba(239,68,68,0.08);
    border: 1px solid #ef4444;
    border-radius: 8px;
    text-align: center;
    color: #f87171;
  }

  .no-https strong {
    display: block;
    font-size: 0.95rem;
    letter-spacing: 0.05em;
    margin-bottom: 0.75rem;
    color: #fca5a5;
  }

  .no-https p {
    font-size: 0.8rem;
    line-height: 1.6;
    color: #888;
  }

  .no-https code {
    color: #06b6d4;
    font-family: inherit;
  }
</style>
