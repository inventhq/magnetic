// ---------------------------------------------------------------------------
// Magnetic CSS — Default design tokens (used when no design.json exists)
// ---------------------------------------------------------------------------

import type { DesignConfig } from './types.ts';

export const defaultDesign: DesignConfig = {
  theme: {
    colors: {
      primary: '#3b82f6',
      secondary: '#8b5cf6',
      accent: '#f59e0b',
      success: '#10b981',
      warning: '#f59e0b',
      error: '#ef4444',
      surface: { light: '#ffffff', dark: '#1a1a2e' },
      text: { light: '#111827', dark: '#f9fafb' },
      muted: { light: '#6b7280', dark: '#9ca3af' },
      border: { light: '#e5e7eb', dark: '#374151' },
    },
    spacing: {
      xs: '0.25rem',
      sm: '0.5rem',
      md: '1rem',
      lg: '1.5rem',
      xl: '2rem',
      '2xl': '3rem',
      '3xl': '4rem',
    },
    radius: {
      sm: '0.25rem',
      md: '0.5rem',
      lg: '1rem',
      full: '9999px',
    },
    typography: {
      sans: 'Inter, system-ui, -apple-system, sans-serif',
      mono: 'JetBrains Mono, ui-monospace, monospace',
      sizes: {
        xs: '0.75rem',
        sm: '0.875rem',
        base: '1rem',
        lg: '1.125rem',
        xl: '1.25rem',
        '2xl': '1.5rem',
        '3xl': '1.875rem',
        '4xl': '2.25rem',
        '5xl': '3rem',
      },
      leading: {
        tight: '1.25',
        normal: '1.5',
        relaxed: '1.75',
      },
    },
    shadows: {
      sm: '0 1px 2px rgb(0 0 0 / 0.05)',
      md: '0 4px 6px rgb(0 0 0 / 0.07), 0 2px 4px rgb(0 0 0 / 0.06)',
      lg: '0 10px 15px rgb(0 0 0 / 0.1), 0 4px 6px rgb(0 0 0 / 0.05)',
      xl: '0 20px 25px rgb(0 0 0 / 0.1), 0 8px 10px rgb(0 0 0 / 0.04)',
    },
    breakpoints: {
      sm: '640px',
      md: '768px',
      lg: '1024px',
      xl: '1280px',
    },
  },
};
