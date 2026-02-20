// types.ts — Internal types for OpenAPI detection and parsing

/** Supported spec versions */
export type SpecVersion = 'openapi3' | 'swagger2';

/** Result of probing a URL for an OpenAPI/Swagger spec */
export interface DiscoveryResult {
  /** Whether a spec was found */
  found: boolean;
  /** URL where the spec was discovered */
  specUrl?: string;
  /** Version of the spec */
  version?: SpecVersion;
  /** Raw spec object (parsed JSON) */
  spec?: any;
  /** Error message if discovery failed */
  error?: string;
}

/** Parsed API endpoint */
export interface ApiEndpoint {
  /** HTTP method (GET, POST, etc.) */
  method: string;
  /** URL path (e.g. /users/{id}) */
  path: string;
  /** Operation ID if available */
  operationId?: string;
  /** Human-readable summary */
  summary?: string;
  /** Request body schema name (if any) */
  requestBody?: string;
  /** Response schema name (if any) */
  responseSchema?: string;
  /** Path parameters */
  pathParams: string[];
  /** Query parameters */
  queryParams: ParamInfo[];
  /** Whether auth is required */
  requiresAuth: boolean;
}

/** Parameter info */
export interface ParamInfo {
  name: string;
  type: string;
  required: boolean;
  description?: string;
}

/** Parsed schema (used for type generation) */
export interface SchemaInfo {
  /** Schema name (e.g. "User", "Post") */
  name: string;
  /** Properties */
  properties: PropertyInfo[];
  /** Required property names */
  required: string[];
  /** Description */
  description?: string;
}

/** Property within a schema */
export interface PropertyInfo {
  name: string;
  type: string;
  /** For arrays, the item type */
  itemType?: string;
  /** For $ref, the referenced schema name */
  ref?: string;
  required: boolean;
  description?: string;
  nullable?: boolean;
  enum?: string[];
}

/** Full parsed API spec */
export interface ParsedApi {
  /** API title */
  title: string;
  /** API version */
  version: string;
  /** Base URL */
  baseUrl: string;
  /** Detected spec version */
  specVersion: SpecVersion;
  /** All endpoints */
  endpoints: ApiEndpoint[];
  /** All schemas/models */
  schemas: SchemaInfo[];
  /** Whether any endpoint requires auth */
  hasAuth: boolean;
}
