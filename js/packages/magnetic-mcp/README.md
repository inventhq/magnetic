# @magneticjs/mcp

MCP (Model Context Protocol) server for the Magnetic framework. Enables AI agents to build, manage, and deploy server-driven UI apps.

## Setup

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "magnetic": {
      "command": "npx",
      "args": ["tsx", "/Users/YOU/path-to/magnetic/js/packages/magnetic-mcp/src/index.ts"]
    }
  }
}
```

### Windsurf / Cursor

Add to your MCP settings:

```json
{
  "magnetic": {
    "command": "npx",
    "args": ["tsx", "/Users/YOU/path-to/magnetic/js/packages/magnetic-mcp/src/index.ts"]
  }
}
```

## Tools

| Tool | Description |
|------|-------------|
| `magnetic_list_files` | List all files in a Magnetic app directory |
| `magnetic_read_file` | Read a file from a Magnetic app (relative paths) |
| `magnetic_write_file` | Write or update a file in a Magnetic app |
| `magnetic_scaffold` | Scaffold a new app (blank or todo template) |
| `magnetic_build` | Build the app bundle (dist/app.js) |
| `magnetic_push` | Build and deploy to the Magnetic platform |
| `magnetic_openapi` | Detect OpenAPI specs from data sources, generate types |
| `magnetic_read_skill` | Read a Magnetic skill document (app-development, components, css-styling) |

## Resources

| URI | Description |
|-----|-------------|
| `magnetic://skills/app-development` | Complete app development guide |
| `magnetic://skills/components` | Component development guide |
| `magnetic://skills/css-styling` | CSS styling guide |
| `magnetic://reference/jsx-runtime` | JSX runtime source code |
| `magnetic://reference/router` | Router source code |
| `magnetic://reference/utilities` | CSS utilities source code |

## Workflow

1. Agent reads `magnetic_read_skill("app-development")` to learn conventions
2. Agent scaffolds with `magnetic_scaffold({ appDir, name, template })`
3. Agent edits files with `magnetic_write_file({ appDir, filePath, content })`
4. Agent builds with `magnetic_build({ appDir })`
5. Agent deploys with `magnetic_push({ appDir })`
