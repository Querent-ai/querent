import { writable, get } from 'svelte/store';
import type { CollectorConfig, Insight, IndexingStatistics } from '../service/bindings';
export const isVisible = writable(false);

function saveToLocalStorage(key: string, value: any) {
	if (typeof window !== 'undefined') {
		localStorage.setItem(key, JSON.stringify(value));
	}
}

function getFromLocalStorage(key: string, defaultValue: any) {
	if (typeof window !== 'undefined') {
		const storedValue = localStorage.getItem(key);
		if (storedValue) {
			return JSON.parse(storedValue);
		}
		return defaultValue;
	}
}

const initialStatePipeline: PipelineState = getFromLocalStorage('pipelineState', {
	mode: 'idle',
	id: null
});

const initialStatePipelinesList: PipelinesData[] = getFromLocalStorage('PipelinesData', []);

export interface CollectorMetadata {
	id: string;
	name: string;
	type: string;
}

interface PipelineState {
	id: string;
	mode: 'idle' | 'running' | 'completed' | 'exited';
}

export interface PipelinesData {
	id: string;
	sources: string[];
	fixed_entities: string[] | undefined;
	sample_entities: string[] | undefined;
	mode: string;
}

export interface DiscoveryData {
	document: string;
	source: string;
	relationship_strength: string;
	sentence: string;
	tags: string;
	top_pairs: string[];
}

export interface APIResponse {
	session_id: string;
	query: string;
	insights: DiscoveryData[];
	page_ranking: number;
}

export interface DiscoveryDataPageList {
	page_number: number;
	data: Insight[];
}

export interface MessageType {
	text: string;
	isUser: boolean;
}

export const describeStats = writable<IndexingStatistics>({
	total_docs: 0,
	total_events: 0,
	total_events_processed: 0,
	total_events_received: 0,
	total_events_sent: 0,
	total_batches: 0,
	total_sentences: 0,
	total_subjects: 0,
	total_predicates: 0,
	total_objects: 0,
	total_graph_events: 0,
	total_vector_events: 0,
	total_data_processed_size: 0
});

const initialMessagesList: MessageType[] = getFromLocalStorage('messagesList', []);
const initialInsightSessionId: string = getFromLocalStorage('insightSessionId', '');
const initialDriveCode: string = getFromLocalStorage('googleDriveCode', '');

export const dataSources = writable<CollectorConfig[]>([]);
export const pipelineState = writable<PipelineState>(initialStatePipeline);
export const pipelineStartTime = writable<number>(0);
export const pipelines = writable<PipelinesData[]>(initialStatePipelinesList);
export const areCollectorsModified = writable(false);
export const discoverylist = writable<DiscoveryDataPageList[]>([]);
export const dropdownDiscovery = writable<DiscoveryData>();
export const firstDiscovery = writable(true);
export const discoveryPageNumber = writable<number>(1);
export const discoveryApiResponseStore = writable<DiscoveryData[]>([]);
export const discoverySessionId = writable<string>();
export const isLoadingInsight = writable<boolean>(false);
export const messagesList = writable<MessageType[]>(initialMessagesList);
export const insightSessionId = writable<string>(initialInsightSessionId);
export const isLicenseVerified = writable(false);
export const statsDataTime = writable<number[]>([]);
export const statsDataTotalEvents = writable<number[]>([]);
export const googleDriveRefreshToken = writable<string>('');
export const discoveryQuery = writable<string>('');

insightSessionId.subscribe(($insightSessionId) => {
	saveToLocalStorage('insightSessionId', $insightSessionId);
});

messagesList.subscribe(($messagesList) => {
	saveToLocalStorage('messagesList', $messagesList);
});

isLoadingInsight.subscribe(($isLoadingInsight) => {
	saveToLocalStorage('isLoading', $isLoadingInsight);
});

discoverySessionId.subscribe(($sessionId) => {
	saveToLocalStorage('discoverySessionId', $sessionId);
});

pipelines.subscribe(($pipelines) => {
	saveToLocalStorage('pipelinesList', $pipelines);
});

dataSources.subscribe(($dataSources) => {
	saveToLocalStorage('dataSources', $dataSources);
});

pipelineState.subscribe(($pipelineState) => {
	saveToLocalStorage('PipelinesData', $pipelineState);
});

discoverylist.subscribe(($discoveryList) => {
	saveToLocalStorage('discoveryList', $discoveryList);
});

dropdownDiscovery.subscribe(($dropdownDiscovery) => {
	saveToLocalStorage('dropdownDiscovery', $dropdownDiscovery);
});

discoveryPageNumber.subscribe(($discoveryPageNumber) => {
	saveToLocalStorage('discoveryPageNumber', $discoveryPageNumber);
});

discoveryApiResponseStore.subscribe(($discoveryApiResponseStore) => {
	saveToLocalStorage('discoveryApiResponseStore', $discoveryApiResponseStore);
});

export function addPipelinesToList(pipeline: PipelinesData): void {
	pipelines.update((currentPipelines) => {
		if (Array.isArray(currentPipelines)) {
			return [...currentPipelines, pipeline];
		} else {
			return [pipeline];
		}
	});
}

export function addDataSource(source: CollectorConfig): void {
	dataSources.update((currentSources) => [...currentSources, source]);
}

export function updatePipeline(mode: PipelineState['mode'], id: PipelineState['id']): void {
	pipelineState.set({ id, mode });
	// reset stats
	describeStats.set({
		total_docs: 0,
		total_events: 0,
		total_events_processed: 0,
		total_events_received: 0,
		total_events_sent: 0,
		total_batches: 0,
		total_sentences: 0,
		total_subjects: 0,
		total_predicates: 0,
		total_objects: 0,
		total_graph_events: 0,
		total_vector_events: 0,
		total_data_processed_size: 0
	});

	// reset stats data
	statsDataTime.set([]);
	statsDataTotalEvents.set([]);
}

export function getCurrentPipelineState(): PipelineState {
	return get(pipelineState);
}
