#!/usr/bin/env node
// @magneticjs/mcp — MCP server for Magnetic framework
// Enables AI agents to build, manage, and deploy Magnetic apps

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';
import { handleTool } from './tools.ts';
import { RESOURCE_DEFINITIONS, readResource } from './resources.ts';

const server = new McpServer(
  {
    name: 'magnetic',
    version: '0.1.0',
  },
  {
    capabilities: {
      tools: {},
      resources: {},
    },
    instructions: `Magnetic MCP Server — Build server-driven UI apps with the Magnetic framework.

Start by reading the "app-development" skill to understand the framework, then use tools to scaffold, build, and deploy apps.

Key workflow:
1. Read skills to understand Magnetic conventions
2. Scaffold a new app with magnetic_scaffold
3. Edit files with magnetic_write_file
4. Build with magnetic_build
5. Deploy with magnetic_push

Important constraints:
- Components are pure functions (no hooks, no state, no effects)
- Events are action name strings, not callbacks
- All state logic lives in server/state.ts
- CSS classes are pre-computed in toViewModel(), not in components
- Use 'class' not 'className' in JSX`,
  }
);

// ── Helper to wrap tool handlers ────────────────────────────────────

function toolHandler(name: string) {
  return async (args: Record<string, any>) => {
    try {
      const result = await handleTool(name, args);
      return { content: [{ type: 'text' as const, text: result }] };
    } catch (err: any) {
      return { content: [{ type: 'text' as const, text: `Error: ${err.message}` }], isError: true };
    }
  };
}

// ── Register Tools with Zod schemas ─────────────────────────────────

server.tool(
  'magnetic_list_files',
  'List all files in a Magnetic app directory. Returns the project structure.',
  { appDir: z.string().describe('Absolute path to the Magnetic app directory') },
  toolHandler('magnetic_list_files'),
);

server.tool(
  'magnetic_read_file',
  'Read a file from a Magnetic app. Use relative paths from the app root (e.g. "pages/IndexPage.tsx", "server/state.ts", "magnetic.json").',
  {
    appDir: z.string().describe('Absolute path to the Magnetic app directory'),
    filePath: z.string().describe('Relative path to the file within the app'),
  },
  toolHandler('magnetic_read_file'),
);

server.tool(
  'magnetic_write_file',
  'Write or update a file in a Magnetic app. Creates parent directories if needed. Use for creating/editing pages, components, state, config, and styles.',
  {
    appDir: z.string().describe('Absolute path to the Magnetic app directory'),
    filePath: z.string().describe('Relative path to the file within the app'),
    content: z.string().describe('File content to write'),
  },
  toolHandler('magnetic_write_file'),
);

server.tool(
  'magnetic_scaffold',
  'Scaffold a new Magnetic app with the standard directory structure. Creates pages/, components/, server/, public/, magnetic.json, design.json, and tsconfig.json.',
  {
    appDir: z.string().describe('Absolute path where the new app should be created'),
    name: z.string().describe('App name (used in magnetic.json and as deployment name)'),
    template: z.enum(['blank', 'todo']).optional().describe('Template: "blank" for empty app, "todo" for a todo list example. Default: blank'),
  },
  toolHandler('magnetic_scaffold'),
);

server.tool(
  'magnetic_build',
  'Build a Magnetic app bundle. Scans pages, generates the V8 bridge, and bundles with esbuild. Output goes to dist/app.js.',
  { appDir: z.string().describe('Absolute path to the Magnetic app directory') },
  toolHandler('magnetic_build'),
);

server.tool(
  'magnetic_push',
  'Build and deploy a Magnetic app to the platform. Requires the app to have a "name" and "server" in magnetic.json or passed as arguments.',
  {
    appDir: z.string().describe('Absolute path to the Magnetic app directory'),
    name: z.string().optional().describe('App name (overrides magnetic.json)'),
    server: z.string().optional().describe('Platform server URL (overrides magnetic.json)'),
  },
  toolHandler('magnetic_push'),
);

server.tool(
  'magnetic_openapi',
  'Detect OpenAPI/Swagger specs from the data sources configured in magnetic.json. If found, generates TypeScript interfaces in server/api-types.ts.',
  { appDir: z.string().describe('Absolute path to the Magnetic app directory') },
  toolHandler('magnetic_openapi'),
);

server.tool(
  'magnetic_read_skill',
  'Read a Magnetic skill document. Skills teach AI agents how to build Magnetic apps. Available skills: "app-development", "components", "css-styling".',
  { skill: z.enum(['app-development', 'components', 'css-styling']).describe('Skill name to read') },
  toolHandler('magnetic_read_skill'),
);

// ── Register Resources ──────────────────────────────────────────────

for (const res of RESOURCE_DEFINITIONS) {
  server.resource(
    res.name,
    res.uri,
    { description: res.description, mimeType: res.mimeType },
    async (uri: URL) => {
      const result = readResource(uri.href);
      if (!result) {
        return {
          contents: [{ uri: uri.href, text: `Resource not found: ${uri.href}`, mimeType: 'text/plain' }],
        };
      }
      return {
        contents: [{ uri: uri.href, text: result.content, mimeType: result.mimeType }],
      };
    }
  );
}

// ── Start ───────────────────────────────────────────────────────────

async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
  process.stderr.write('[magnetic-mcp] Server started\n');
}

main().catch((err) => {
  process.stderr.write(`[magnetic-mcp] Fatal: ${err.message}\n`);
  process.exit(1);
});
