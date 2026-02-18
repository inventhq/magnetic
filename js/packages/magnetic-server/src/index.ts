// @magnetic/server — Magnetic Server Components
// Re-exports for developer use

export type { DomNode } from './jsx-runtime.ts';
export { Link, Head } from './jsx-runtime.ts';

export { createRouter, renderRoute, navigateAction } from './router.ts';
export type { Router, RouteDefinition, RouteMatch, RouteResult, RouteGuard, PageComponent, LayoutComponent } from './router.ts';

export { scanPages, fileNameToSegment } from './file-router.ts';
export type { FileRouterOptions } from './file-router.ts';

export { renderToHTML, renderPage, extractHead } from './ssr.ts';
export type { PageOptions, ExtractedHead } from './ssr.ts';

export { withErrorBoundary, safeReduce, defaultFallback } from './error-boundary.ts';
export type { ErrorFallback, ErrorFallbackProps } from './error-boundary.ts';

export { createMiddleware, createContext, loggerMiddleware, corsMiddleware, rateLimitMiddleware } from './middleware.ts';
export type { MiddlewareStack, MiddlewareFn, MagneticContext, NextFn } from './middleware.ts';

export { buildAssets, saveManifest, loadManifest, createAssetResolver, serveStatic } from './assets.ts';
export type { AssetManifest, StaticFileResult } from './assets.ts';
