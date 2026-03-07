---
description: Deploy a documentation site from a folder of markdown files as a Magnetic SSG app
---

# Deploy Docs Site

Turn a folder of `.md` files into a live documentation site with sidebar navigation, dark theme, and chapter ordering.

## Prerequisites

- The markdown files exist in an `apps/<name>/` folder
- The Magnetic CLI source is at `js/packages/magnetic-cli/src/cli.ts`
- You have deploy access to `https://api.fujs.dev`

## Steps

### 1. Identify the source folder and read the index file

Find the app folder under `apps/`. Read `index.md` (or equivalent landing page) to understand the chapter ordering. The index file usually has a "Chapters" or "Documentation Map" section listing files in order.

### 2. Create the app directory structure

```
apps/<name>/
├── content/          ← markdown files go here (with frontmatter)
├── pages/
│   ├── IndexPage.tsx
│   └── [slug].tsx
├── server/
│   └── state.ts
├── public/
│   └── style.css
├── magnetic.json
├── design.json
├── tsconfig.json
└── .gitignore
```

### 3. Move markdown files into `content/` and add frontmatter

The content pipeline requires YAML frontmatter with `title` and `order` fields. Raw markdown files without frontmatter will not sort or display correctly.

```bash
mkdir -p content pages server public
```

For each `.md` file, copy it to `content/` and prepend frontmatter:

```bash
# Rename index.md → introduction.md (it becomes the homepage)
cp index.md content/introduction.md

# Copy all other .md files (skip README.md)
for f in *.md; do
  [ "$f" = "README.md" ] && continue
  [ "$f" = "index.md" ] && continue
  cp "$f" content/
done
```

Then prepend frontmatter to each file. The `order` field controls sidebar position (0 = top). Use the chapter ordering from the index file.

```bash
prepend() {
  local f="$1" t="$2" o="$3"
  local tmp=$(mktemp)
  printf -- "---\ntitle: %s\norder: %s\n---\n\n" "$t" "$o" > "$tmp"
  cat "$f" >> "$tmp"
  mv "$tmp" "$f"
}

prepend content/introduction.md "Product Name" 0
prepend content/getting-started.md "Getting Started" 1
prepend content/concepts.md "Core Concepts" 2
# ... etc, one per file, matching the chapter order from the index
```

**Critical**: `order: 0` is valid. The sort uses `??` (nullish coalescing), not `||`.

**Critical**: After adding frontmatter, fix all internal markdown links. Replace `(./filename.md)` with `(/filename)` — the SSG site uses route paths, not file paths. Links ending in `.md` will 404.

```bash
# Bulk-fix all .md links in content files
cd content
sed -i '' 's|(./index.md)|(/)|g' *.md
# Repeat for every content file slug:
sed -i '' 's|(./getting-started.md)|(/getting-started)|g' *.md
# ... etc for each file
```

### 4. Create `pages/IndexPage.tsx`

Copy from `apps/docs/pages/IndexPage.tsx` and customize:
- Change the `<title>` suffix (e.g., `— Product Docs`)
- Change the sidebar brand `<h1>` (e.g., `Product Name`)
- Change the sidebar tagline `<span>` (e.g., `Your Tagline`)
- Update external links in the sidebar footer

### 5. Create `pages/[slug].tsx`

This is always identical:

```tsx
// Dynamic route: /:slug — renders the same page as /
import { IndexPage } from './IndexPage.tsx';
export const _slug_Page = IndexPage;
```

### 6. Create `server/state.ts`

Copy from `apps/docs/server/state.ts`. The only thing to customize is the fallback title string (e.g., `'Product Docs'` instead of `'Magnetic Docs'`).

Key details in state.ts that MUST be correct:
- Default slug is `'introduction'` (matches the renamed index.md)
- Sort uses `(a.meta.order ?? 99)` — NOT `||` (which treats 0 as falsy)
- Introduction sidebar entry links to `/` not `/introduction`:
  ```ts
  slug: doc.slug === 'introduction' ? '' : doc.slug,
  href: doc.slug === 'introduction' ? '/' : '/' + doc.slug,
  ```

### 7. Create config files

**`magnetic.json`**:
```json
{
  "name": "<app-name>",
  "server": "https://api.fujs.dev"
}
```

**`design.json`**: Copy from `apps/docs/design.json` (dark theme with indigo accent).

**`tsconfig.json`**: Copy from `apps/docs/tsconfig.json`.

**`.gitignore`**:
```
node_modules/
app.js
dist/app.js
```

**`public/style.css`**: Copy from `apps/docs/public/style.css` (sidebar layout + prose styles).

### 8. Build and deploy

// turbo
```bash
npx tsx js/packages/magnetic-cli/src/cli.ts push --static --dir apps/<name>
```

Run from the repo root (`/Users/vv/Desktop/magnetic`) or set `--dir` to the absolute path.

The output will show:
- How many markdown files were found (should match your content/ count)
- Pre-rendered routes (one per content file + the index)
- The live URL (e.g., `https://<id>.fujs.dev`)

### 9. Commit and push

```bash
git add apps/<name>
GIT_EDITOR=true git commit -m "deploy <name> docs site"
git push
```

## Common Issues

| Problem | Cause | Fix |
|---------|-------|-----|
| Sidebar items in wrong order | Missing or wrong `order` in frontmatter | Check each file has `order: N` in YAML frontmatter |
| Introduction at bottom of sidebar | `order: 0` treated as falsy | Ensure state.ts uses `??` not `||` in sort |
| Homepage shows "Page not found" | Default slug doesn't match filename | Ensure `index.md` was renamed to `introduction.md` |
| No content files found | Files not in `content/` subdirectory | Move .md files from root into `content/` |
| Frontmatter showing as raw text | Missing `---` delimiters | Ensure file starts with `---\n` on first line |

## Reference Implementations

- `apps/docs/` — Magnetic framework docs (7 chapters)
- `apps/plugin-runtime-docs/` — Plugin Runtime docs (9 chapters)
- `apps/bitbin-docs/` — BitBin docs (13 chapters)
