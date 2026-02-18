// @magnetic/server — Error Boundaries
// Wraps render/reduce in try-catch with fallback UI

import type { DomNode } from './jsx-runtime.ts';

export interface ErrorFallbackProps {
  error: Error;
  path?: string;
  action?: string;
}

export type ErrorFallback = (props: ErrorFallbackProps) => DomNode;

/** Default fallback — minimal error display */
export const defaultFallback: ErrorFallback = ({ error, action }) => ({
  tag: 'div',
  key: 'error-boundary',
  attrs: { class: 'magnetic-error' },
  children: [
    { tag: 'h2', key: 'err-h', text: 'Something went wrong' },
    { tag: 'p', key: 'err-msg', text: error.message },
    ...(action ? [{ tag: 'p', key: 'err-act', text: `Action: ${action}` }] : []),
  ],
});

/**
 * Wraps a render function with error handling.
 * If render throws, the fallback component is returned instead.
 */
export function withErrorBoundary<T extends (...args: any[]) => DomNode>(
  renderFn: T,
  fallback: ErrorFallback = defaultFallback,
): T {
  return ((...args: any[]) => {
    try {
      return renderFn(...args);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      console.error('[magnetic] render error:', error.message);
      return fallback({ error });
    }
  }) as T;
}

/**
 * Wraps a reducer with error handling.
 * If reduce throws, the original state is returned unchanged.
 */
export function safeReduce<S>(
  reduceFn: (state: S, action: string, payload: any) => S,
  onError?: (error: Error, action: string) => void,
): (state: S, action: string, payload: any) => S {
  return (state, action, payload) => {
    try {
      return reduceFn(state, action, payload);
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err));
      console.error(`[magnetic] reduce error on "${action}":`, error.message);
      if (onError) onError(error, action);
      return state;
    }
  };
}
