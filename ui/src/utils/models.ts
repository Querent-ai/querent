

/* eslint-disable  @typescript-eslint/no-explicit-any */
export type RawDoc = Record<string, any>

export type FieldMapping = {
  description: string | null;
  name: string;
  type: string;
  stored: boolean | null;
  fast: boolean | null;
  indexed: boolean | null;
  // Specific datetime field attributes.
  output_format: string | null;
  field_mappings?: FieldMapping[];
}

export type Field = {
  // Json path (path segments concatenated as a string with dots between segments).
  json_path: string;
  // Json path of the field.
  path_segments: string[];
  field_mapping: FieldMapping;
}

export type Entry = {
  key: string;
  value: any;
}

export const DATE_TIME_WITH_SECONDS_FORMAT = "YYYY/MM/DD HH:mm:ss";
export const DATE_TIME_WITH_MILLISECONDS_FORMAT = "YYYY/MM/DD HH:mm:ss.SSS";

// Returns a flatten array of fields and nested fields found in the given `FieldMapping` array. 
export function getAllFields(field_mappings: Array<FieldMapping>): Field[] {
  const fields: Field[] = [];
  for (const field_mapping of field_mappings) {
    if (field_mapping.type === 'object' && field_mapping.field_mappings !== undefined) {
      for (const child_field_mapping of getAllFields(field_mapping.field_mappings)) {
        fields.push({json_path: field_mapping.name + '.' + child_field_mapping.json_path, path_segments: [field_mapping.name].concat(child_field_mapping.path_segments), field_mapping: child_field_mapping.field_mapping})
      }
    } else {
      fields.push({json_path: field_mapping.name, path_segments: [field_mapping.name], field_mapping: field_mapping});
    }
  }

  return fields;
}

export type DocMapping = {
  field_mappings: FieldMapping[];
  tag_fields: string[];
  store: boolean;
  dynamic_mapping: boolean;
  timestamp_field: string | null;
}

export type SortOrder = 'Asc' | 'Desc';

export type SortByField = {
  field_name: string,
  order: SortOrder
}

export type SearchRequest = {
  indexId: string | null;
  query: string;
  startTimestamp: number | null;
  endTimestamp: number | null;
  maxHits: number;
  sortByField: SortByField | null;
}

export const EMPTY_SEARCH_REQUEST: SearchRequest = {
  indexId: '',
  query: '',
  startTimestamp: null,
  endTimestamp: null,
  maxHits: 100,
  sortByField: null,
}

export type ResponseError = {
  status: number | null;
  message: string | null;
}

export type SearchResponse = {
  num_hits: number;
  hits: Array<RawDoc>;
  elapsed_time_micros: number;
  errors: Array<any> | undefined;
}

export type IndexConfig = {
  version: string;
  index_id: string;
  index_uri: string;
  doc_mapping: DocMapping;
  indexing_settings: object;
  search_settings: object;
  retention: object;
}

export type IndexMetadata = {
  index_config: IndexConfig;
  checkpoint: object;
  sources: object[] | undefined;
  create_timestamp: number;
}

export const EMPTY_INDEX_METADATA: IndexMetadata = {
  index_config: {
    version: '',
    index_uri: '',
    index_id: '',
    doc_mapping: {
      field_mappings: [],
      tag_fields: [],
      store: false,
      dynamic_mapping: false,
      timestamp_field: null
    },
    indexing_settings: {},
    search_settings: {},
    retention: {},
  },
  checkpoint: {},
  sources: undefined,
  create_timestamp: 0,
};

export type SplitMetadata = {
  split_id: string;
  split_state: string;
  num_docs: number;
  uncompressed_docs_size_in_bytes: number;
  time_range: null | Range;
  update_timestamp: number;
  version: number;
  create_timestamp: number;
  tags: string[];
  demux_num_ops: number;
  footer_offsets: Range;
}

export type Range = {
  start: number;
  end: number;
}

export type Index = {
  metadata: IndexMetadata;
  splits: SplitMetadata[];
}

export type Cluster = {
  node_id: string,
  cluster_id: string,
  state: ClusterState,
}

export type ClusterState = {
  state: ClusterStateSnapshot;
  live_nodes: any[];
  dead_nodes: any[];
}

export type ClusterStateSnapshot = {
  seed_addrs: string[],
  node_states: Record<string, NodeState>,
}

export type NodeState = {
  key_values: KeyValues,
  max_version: number,
}

export type KeyValues = {
  available_services: KeyValue,
  grpc_address: KeyValue,
  heartbeat: KeyValue,
}

export type KeyValue = {
  value: any,
  version: number,
}

export type QuesterBuildInfo = {
  commit_version_tag: string,
  cargo_pkg_version: string,
  cargo_build_target: string,
  commit_short_hash: string,
  commit_date: string,
  version: string,
}

export type NodeId = {
  id: string,
  grpc_address: string,
  self: boolean,
}
export interface SemanticServiceCounters {
  num_failed_pipelines: number;
  num_running_pipelines: number;
  num_successful_pipelines: number;
}

export type PipelinesMetadata = {
  pipelines: Array<{
    attr: string;
    import: string;
    name: string;
    pipeline_id: string;
  }>;
};

export interface IndexingStatistics {
  total_batches: number;
  total_docs: number;
  total_events: number;
  total_events_processed: number;
  total_events_received: number;
  total_events_sent: number;
  total_graph_events: number;
  total_graph_events_sent: number;
  total_objects: number;
  total_predicates: number;
  total_semantic_knowledge: number;
  total_sentences: number;
  total_subjects: number;
  total_vector_events: number;
  total_vector_events_sent: number;
}

export interface RestConfig {
  listen_port: number;
  cors_allow_origins: string[];
  extra_headers: Record<string, string>;
}

export interface StorageConfig {
  name: string;
  storage_type: string;
  config: Record<string, any>; // Adjust this if you know specific keys and types
}

export interface JaegerConfig {
  enable_endpoint: boolean;
  lookback_period_hours: number;
  max_trace_duration_secs: number;
  max_fetch_spans: number;
}

export interface NodeConfig {
  cluster_id: string;
  node_id: string;
  listen_address: string;
  advertise_address: string;
  gossip_listen_port: number;
  grpc_listen_port: number;
  rest_config: RestConfig;
  peer_seeds: string[];
  cpu_capacity: number;
  storage_configs: Record<string, StorageConfig>;
  tracing: {
    jaeger: JaegerConfig;
  };
}

