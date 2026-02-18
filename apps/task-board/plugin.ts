// plugin.ts — Task Board as a Magnetic plugin for @inventhq/plugin-sdk
//
// This is the entry point that esbuild bundles into a single JS file.
// The output runs inside a V8 isolate with definePlugin/RuntimeHelper as globals.

import { createMagneticPlugin } from '../../js/packages/magnetic-plugin-adapter/src/index.ts';
import { App } from './components/App.tsx';
import { initialState, reduce, toViewModel } from './server/state.ts';
import type { AppState } from './server/state.ts';

// definePlugin is a global in the plugin-sdk V8 isolate
declare const definePlugin: (config: any) => any;

export default definePlugin(createMagneticPlugin<AppState>({
  name: "task-board",
  initialState,
  reduce,
  render: (state) => App(toViewModel(state)),
}));
