<script>
  import { generateIdentity, saveIdentity } from '../lib/auth.js';
  import { registerWithCredentials } from '../lib/api.js';
  import { encryptSecretKey } from '../lib/crypto.js';
  import { identity as identityStore } from '../lib/store.js';
  import { navigate } from '../lib/router.js';

  const secureContext = window.isSecureContext && crypto?.subtle != null;

  let step = 'code'; // code | credentials | generating | done
  let accessCode = '';
  let username = '';
  let password = '';
  let passwordConfirm = '';
  let errorMsg = '';

  function goToCredentials() {
    errorMsg = '';
    if (!accessCode.trim()) return;
    step = 'credentials';
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

    {#if !secureContext}
      <div class="no-https">
        <strong>HTTPS required</strong>
        <p>This app requires a secure connection. Visit <code>https://webapp.questlinecli.com</code></p>
      </div>

    {:else if step === 'code'}
      <form on:submit|preventDefault={goToCredentials} class="form">
        <div class="field">
          <label for="code">Access Code</label>
          <input
            id="code"
            type="text"
            bind:value={accessCode}
            placeholder="XXXX-XXXX-XXXX-XXXX"
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        <button type="submit" class="btn-primary" disabled={!accessCode.trim()}>
          Continue →
        </button>
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

        <div class="field">
          <label for="username">Username</label>
          <input
            id="username"
            type="text"
            bind:value={username}
            placeholder="your_username"
            autocomplete="username"
            spellcheck="false"
            maxlength="50"
          />
        </div>

        <div class="field">
          <label for="password">Password</label>
          <input
            id="password"
            type="password"
            bind:value={password}
            placeholder="at least 8 characters"
            autocomplete="new-password"
          />
        </div>

        <div class="field">
          <label for="passwordConfirm">Confirm Password</label>
          <input
            id="passwordConfirm"
            type="password"
            bind:value={passwordConfirm}
            placeholder="repeat password"
            autocomplete="new-password"
          />
        </div>

        {#if errorMsg}
          <div class="error">{errorMsg}</div>
        {/if}

        <button type="submit" class="btn-primary" disabled={!username.trim() || !password || !passwordConfirm}>
          Forge Identity
        </button>

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
    font-family: 'JetBrains Mono', monospace;
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

  /* ── Logo — outside card, large, matches index.html #logo-img ── */
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
    background: rgba(8,8,8,0.88);
    border: 1px solid #1c1c1c;
    border-radius: 14px;
    padding: 2rem 2.5rem;
    text-align: center;
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    box-shadow: 0 0 60px rgba(0,0,0,0.6);
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
    font-size: 0.75rem;
    color: #666;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    margin-bottom: 0.5rem;
  }

  input {
    width: 100%;
    background: #050505;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.9rem;
    padding: 0.65rem 0.9rem;
    outline: none;
    letter-spacing: 0.1em;
    transition: border-color 0.15s;
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
    letter-spacing: 0.1em;
    padding: 0.75rem;
    cursor: pointer;
    text-transform: uppercase;
    transition: background 0.15s, color 0.15s;
  }

  .btn-primary:hover:not(:disabled) {
    background: rgba(168,85,247,0.28);
    color: #c084fc;
    box-shadow: 0 0 16px rgba(168,85,247,0.2);
  }

  .btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

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
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 2px solid #2a2a2a;
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
