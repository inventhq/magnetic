# task-board

A [Magnetic](https://github.com/example/magnetic) server-driven UI app.

## Quick Start

```bash
bash scripts/dev.sh
```

Then open http://localhost:3000

## Structure

- `rs/src/main.rs` — Rust server + reducer logic
- `components/` — .magnetic.html templates
- `public/` — Static files served to the client
- `public/magnetic.js` — Magnetic client runtime (~1.6KB gzipped)
