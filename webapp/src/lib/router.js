// Minimal client-side router using history.pushState

import { writable } from 'svelte/store';

export const route = writable(window.location.pathname);

export function navigate(path) {
  window.history.pushState({}, '', path);
  route.set(path);
}

// Handle browser back/forward
window.addEventListener('popstate', () => {
  route.set(window.location.pathname);
});
