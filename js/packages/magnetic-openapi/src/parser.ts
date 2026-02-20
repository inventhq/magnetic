// parser.ts — Parse OpenAPI 3.x / Swagger 2.x specs into structured data

import type {
  SpecVersion, ParsedApi, ApiEndpoint, SchemaInfo, PropertyInfo, ParamInfo,
} from './types.ts';

/** Resolve a $ref string to a schema name: "#/components/schemas/User" → "User" */
function refName(ref: string): string {
  const parts = ref.split('/');
  return parts[parts.length - 1];
}

/** Map OpenAPI type strings to TypeScript type strings */
function mapType(schema: any): { type: string; itemType?: string; ref?: string } {
  if (!schema) return { type: 'any' };

  if (schema.$ref) {
    return { type: refName(schema.$ref), ref: refName(schema.$ref) };
  }

  switch (schema.type) {
    case 'string':   return { type: schema.enum ? schema.enum.map((e: string) => `'${e}'`).join(' | ') : 'string' };
    case 'integer':  return { type: 'number' };
    case 'number':   return { type: 'number' };
    case 'boolean':  return { type: 'boolean' };
    case 'array': {
      const items = mapType(schema.items);
      return { type: `${items.type}[]`, itemType: items.type, ref: items.ref };
    }
    case 'object': {
      if (schema.additionalProperties) {
        const valType = mapType(schema.additionalProperties);
        return { type: `Record<string, ${valType.type}>` };
      }
      return { type: 'Record<string, any>' };
    }
    default:
      if (schema.oneOf || schema.anyOf) {
        const variants = (schema.oneOf || schema.anyOf).map((s: any) => mapType(s).type);
        return { type: variants.join(' | ') };
      }
      return { type: 'any' };
  }
}

/** Parse a schema object into SchemaInfo */
function parseSchema(name: string, schema: any): SchemaInfo {
  const required = new Set<string>(schema.required || []);
  const properties: PropertyInfo[] = [];

  if (schema.properties) {
    for (const [propName, propSchema] of Object.entries(schema.properties)) {
      const ps = propSchema as any;
      const mapped = mapType(ps);
      properties.push({
        name: propName,
        type: mapped.type,
        itemType: mapped.itemType,
        ref: mapped.ref,
        required: required.has(propName),
        description: ps.description,
        nullable: ps.nullable,
        enum: ps.enum,
      });
    }
  }

  // Handle allOf (common in OpenAPI 3.x for inheritance)
  if (schema.allOf) {
    for (const sub of schema.allOf) {
      if (sub.properties) {
        const subRequired = new Set<string>(sub.required || []);
        for (const [propName, propSchema] of Object.entries(sub.properties)) {
          const ps = propSchema as any;
          const mapped = mapType(ps);
          properties.push({
            name: propName,
            type: mapped.type,
            itemType: mapped.itemType,
            ref: mapped.ref,
            required: subRequired.has(propName) || required.has(propName),
            description: ps.description,
            nullable: ps.nullable,
            enum: ps.enum,
          });
        }
      }
    }
  }

  return {
    name,
    properties,
    required: Array.from(required),
    description: schema.description,
  };
}

/** Check if an operation requires auth */
function operationRequiresAuth(op: any, globalSecurity: any[]): boolean {
  if (op.security) return op.security.length > 0 && Object.keys(op.security[0] || {}).length > 0;
  if (globalSecurity) return globalSecurity.length > 0 && Object.keys(globalSecurity[0] || {}).length > 0;
  return false;
}

/** Extract path parameters from a path string: /users/{id} → ["id"] */
function extractPathParams(path: string): string[] {
  const matches = path.match(/\{([^}]+)\}/g);
  if (!matches) return [];
  return matches.map(m => m.slice(1, -1));
}

/** Get the response schema for a successful response */
function getResponseSchema(op: any, version: SpecVersion): string | undefined {
  const responses = op.responses;
  if (!responses) return undefined;

  const success = responses['200'] || responses['201'] || responses['default'];
  if (!success) return undefined;

  if (version === 'openapi3') {
    const content = success.content;
    if (!content) return undefined;
    const json = content['application/json'];
    if (!json || !json.schema) return undefined;
    if (json.schema.$ref) return refName(json.schema.$ref);
    if (json.schema.type === 'array' && json.schema.items?.$ref) {
      return refName(json.schema.items.$ref) + '[]';
    }
    return undefined;
  }

  // Swagger 2.x
  if (success.schema) {
    if (success.schema.$ref) return refName(success.schema.$ref);
    if (success.schema.type === 'array' && success.schema.items?.$ref) {
      return refName(success.schema.items.$ref) + '[]';
    }
  }
  return undefined;
}

/** Get the request body schema name */
function getRequestBody(op: any, version: SpecVersion): string | undefined {
  if (version === 'openapi3') {
    const rb = op.requestBody;
    if (!rb) return undefined;
    const content = rb.content;
    if (!content) return undefined;
    const json = content['application/json'];
    if (!json || !json.schema) return undefined;
    if (json.schema.$ref) return refName(json.schema.$ref);
    return undefined;
  }

  // Swagger 2.x — body params
  const bodyParam = (op.parameters || []).find((p: any) => p.in === 'body');
  if (bodyParam?.schema?.$ref) return refName(bodyParam.schema.$ref);
  return undefined;
}

/** Parse query parameters from an operation */
function getQueryParams(op: any, version: SpecVersion): ParamInfo[] {
  const params: ParamInfo[] = [];

  const rawParams = op.parameters || [];
  for (const p of rawParams) {
    if (p.in !== 'query') continue;

    if (version === 'openapi3') {
      const schema = p.schema || {};
      params.push({
        name: p.name,
        type: mapType(schema).type,
        required: p.required === true,
        description: p.description,
      });
    } else {
      params.push({
        name: p.name,
        type: mapType(p).type,
        required: p.required === true,
        description: p.description,
      });
    }
  }

  return params;
}

/**
 * Parse an OpenAPI 3.x or Swagger 2.x spec into structured data.
 */
export function parse(spec: any, version: SpecVersion): ParsedApi {
  // Extract title and version
  const info = spec.info || {};
  const title = info.title || 'Untitled API';
  const apiVersion = info.version || '0.0.0';

  // Extract base URL
  let baseUrl = '';
  if (version === 'openapi3') {
    const servers = spec.servers || [];
    if (servers.length > 0) baseUrl = servers[0].url || '';
  } else {
    const host = spec.host || '';
    const basePath = spec.basePath || '';
    const scheme = (spec.schemes || ['https'])[0];
    if (host) baseUrl = `${scheme}://${host}${basePath}`;
  }

  // Extract schemas
  const schemas: SchemaInfo[] = [];
  const rawSchemas = version === 'openapi3'
    ? (spec.components?.schemas || {})
    : (spec.definitions || {});

  for (const [name, schema] of Object.entries(rawSchemas)) {
    schemas.push(parseSchema(name, schema));
  }

  // Extract endpoints
  const endpoints: ApiEndpoint[] = [];
  const globalSecurity = spec.security || [];
  const paths = spec.paths || {};

  for (const [path, pathItem] of Object.entries(paths)) {
    const pi = pathItem as any;
    const methods = ['get', 'post', 'put', 'patch', 'delete', 'head', 'options'];

    for (const method of methods) {
      const op = pi[method];
      if (!op) continue;

      endpoints.push({
        method: method.toUpperCase(),
        path,
        operationId: op.operationId,
        summary: op.summary || op.description,
        requestBody: getRequestBody(op, version),
        responseSchema: getResponseSchema(op, version),
        pathParams: extractPathParams(path),
        queryParams: getQueryParams(op, version),
        requiresAuth: operationRequiresAuth(op, globalSecurity),
      });
    }
  }

  return {
    title,
    version: apiVersion,
    baseUrl,
    specVersion: version,
    endpoints,
    schemas,
    hasAuth: endpoints.some(e => e.requiresAuth),
  };
}
