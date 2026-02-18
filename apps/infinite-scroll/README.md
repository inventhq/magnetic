# Magnetic — Virtualized Infinite Scroll

A benchmark app demonstrating server-driven virtualization with 1500+ variable-height cards.

## Architecture

- **Server** holds all 1500+ feed items in memory
- **Client** sends scroll position to server on every scroll event (throttled 50ms)
- **Server** computes visible window via binary search + overscan, sends only ~15-20 visible cards
- **DOM** contains only visible cards + two spacer divs for correct scroll height
- **No virtual DOM**, no client-side virtualization library

## Run

```bash
bash scripts/dev.sh           # 1500 items, port 3001
bash scripts/dev.sh 5000 3002 # 5000 items, port 3002
```

## Benchmarking

The overlay in the top-right shows:
- **FPS** — current and rolling average (target: 60fps)
- **Frame time** — ms per frame (target: <16.7ms)
- **TTI** — Time to Interactive from page load
- **SSR→JS** — time from SSR HTML visible to JS takeover
- **DOM nodes** — total elements in DOM (should stay flat ~200-400 regardless of list size)
- **JS heap** — memory usage (Chrome only)
- **Total/Visible/Window** — items in dataset vs rendered vs index range
