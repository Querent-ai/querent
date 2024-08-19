import { writable, get } from 'svelte/store';
import type { Insight } from '../service/bindings';
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

export function clearDataSources(): void {
	dataSources.set([]);
	saveToLocalStorage('dataSources', []);
}

const initialStateDataSources: CollectorMetadata[] = getFromLocalStorage('dataSources', []);
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
	fixed_entities: string[];
	sample_entities: string[];
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

export const dataSources = writable<CollectorMetadata[]>(initialStateDataSources);
export const pipelineState = writable<PipelineState>(initialStatePipeline);
export const pipelines = writable<PipelinesData[]>(initialStatePipelinesList);
export const areCollectorsModified = writable(false);
export const discoverylist = writable<DiscoveryDataPageList[]>([]);
export const dropdownDiscovery = writable<DiscoveryData>();
export const firstDiscovery = writable(true);
export const discoveryPageNumber = writable<number>(1);
export const discoveryApiResponseStore = writable<DiscoveryData[]>([]);

pipelines.subscribe(($pipelines) => {
	saveToLocalStorage('pipelinesList', $pipelines);
});

dataSources.subscribe(($dataSources) => {
	saveToLocalStorage('dataSources', $dataSources);
});

pipelineState.subscribe(($pipelineState) => {
	saveToLocalStorage('PipelinesData', $pipelineState);
});

pipelineState.subscribe(($discoveryList) => {
	saveToLocalStorage('discoveryList', $discoveryList);
});

pipelineState.subscribe(($dropdownDiscovery) => {
	saveToLocalStorage('dropdownDiscovery', $dropdownDiscovery);
});

pipelineState.subscribe(($discoveryPageNumber) => {
	saveToLocalStorage('discoveryPageNumber', $discoveryPageNumber);
});

pipelineState.subscribe(($discoveryApiResponseStore) => {
	saveToLocalStorage('discoveryApiResponseStore', $discoveryApiResponseStore);
});

export function addPipelinesToList(pipeline: PipelinesData): void {
	pipelines.update((currentPipelines) => [...currentPipelines, pipeline]);
}

export function addDataSource(source: CollectorMetadata): void {
	dataSources.update((currentSources) => [...currentSources, source]);
}

export function updatePipeline(mode: PipelineState['mode'], id: PipelineState['id']): void {
	pipelineState.set({ id, mode });
}

export function getCurrentDataSources(): CollectorMetadata[] {
	return get(dataSources);
}

export function getCurrentPipelineState(): PipelineState {
	return get(pipelineState);
}

export function deleteSourcefromList(id: string): void {
	dataSources.update((currentSources) => {
		const updatedSources = currentSources.filter((source) => source.id !== id);
		saveToLocalStorage('dataSources', updatedSources);
		return updatedSources;
	});
}

export function countSourcesByType(type: string): number {
	const sources = get(dataSources);
	if (sources && Array.isArray(sources)) {
		return sources.filter((source) => source.type === type).length;
	}
	return 0;
}
