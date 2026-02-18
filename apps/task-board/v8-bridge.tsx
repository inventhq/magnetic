// v8-bridge.tsx — Entry point for the Rust V8 server
// Bundles as IIFE, exposes globalThis.MagneticApp = { render, reduce }
// Built with: esbuild --bundle --format=iife --global-name=MagneticApp

import { createRouter } from '../../js/packages/magnetic-server/src/router.ts';
import { TasksPage } from './pages/TasksPage.tsx';
import { AboutPage } from './pages/AboutPage.tsx';
import { NotFoundPage } from './pages/NotFoundPage.tsx';
import { initialState, reduce as appReduce, toViewModel } from './server/state.ts';
import type { AppState } from './server/state.ts';

const router = createRouter([
  { path: '/', page: TasksPage },
  { path: '/about', page: AboutPage },
  { path: '*', page: NotFoundPage },
]);

let state: AppState = initialState();

/** Render the current state for a given path → DomNode */
export function render(path: string) {
  const vm = toViewModel(state);
  const result = router.resolve(path, vm);
  if (!result) return NotFoundPage({ params: {} });
  if (result.kind === 'redirect') {
    const r2 = router.resolve(result.to, vm);
    if (r2 && r2.kind === 'render') return r2.dom;
    return NotFoundPage({ params: {} });
  }
  return result.dom;
}

/** Reduce an action → updates internal state, returns new DomNode for current path */
export function reduce(actionPayload: { action: string; payload?: any; path?: string }) {
  const { action, payload = {}, path = '/' } = actionPayload;
  state = appReduce(state, action, payload);
  return render(path);
}
