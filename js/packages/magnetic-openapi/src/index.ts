// @magneticjs/openapi — OpenAPI/Swagger detection and type generation for Magnetic

export { discover, discoverAll } from './discover.ts';
export { parse } from './parser.ts';
export { generateTypes, suggestDataSources } from './codegen.ts';
export type {
  SpecVersion,
  DiscoveryResult,
  ApiEndpoint,
  ParamInfo,
  SchemaInfo,
  PropertyInfo,
  ParsedApi,
} from './types.ts';
