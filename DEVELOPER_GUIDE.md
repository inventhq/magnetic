# Magnetic Developer Guide

Magnetic is a **server-driven UI framework**. You write TSX pages and business logic — the server renders everything. No React, no virtual DOM, no hydration. Web, iOS, and Android from one codebase.

## Quick Start

```bash
npm install -g @magneticjs/cli
npx create-magnetic-app my-app
cd my-app
magnetic dev          # → http://localhost:3003
magnetic push --name my-app --server https://api.fujs.dev   # deploy
```

## Documentation

### For App Developers (building with Magnetic)
Start here → `apps/docs/content/introduction.md`

| Chapter | File | What |
|---------|------|------|
| Introduction | `apps/docs/content/introduction.md` | What is Magnetic, architecture, key concepts |
| Getting Started | `apps/docs/content/getting-started.md` | Install, scaffold, first app |
| App Development | `apps/docs/content/app-development.md` | Pages, state, actions, data sources, config |
| Components | `apps/docs/content/components.md` | Component patterns, built-ins |
| CSS & Styling | `apps/docs/content/css-styling.md` | Design tokens, utility classes |
| Deployment | `apps/docs/content/deployment.md` | SSR, SSG, hybrid pre-render |
| Benchmarks | `apps/docs/content/benchmarks.md` | Performance data |

Live docs: [blbetmes.fujs.dev](https://blbetmes.fujs.dev)

### For Core Developers (extending the platform)
Start here → `docs/index.md`

| Chapter | File | What |
|---------|------|------|
| Overview | `docs/index.md` | Architecture overview, key source files |
| Architecture | `docs/architecture.md` | Platform server internals, V8, request flow |
| Packages | `docs/packages.md` | npm packages, native binary, publishing |
| Deployment & Ops | `docs/deployment-ops.md` | Infrastructure, Caddy, production fixes |
| Roadmap | `docs/roadmap.md` | CRDTs, R2 content storage, future work |
