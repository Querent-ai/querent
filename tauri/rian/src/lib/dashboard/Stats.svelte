<script lang="ts">
	import { Card } from 'flowbite-svelte';
	import { commands, type IndexingStatistics } from '../../service/bindings';
	import { onDestroy, onMount } from 'svelte';
	import { describeStats } from '../../stores/appState';

	let selectedPipeline: string;
	let runningPipelines: string[] = [];
	$: selectedPipeline = runningPipelines.length > 0 ? runningPipelines[0] : 'no_active_pipeline';
	let isLive = false;
	onMount(async () => {
		try {
			let res = await commands.getRunningAgns();
			runningPipelines = res.map(([pipelineId, _]) => pipelineId);
			isLive = runningPipelines.length > 0;

			if ($describeStats.total_docs > 0) {
				productsArray = convertStatsToArray($describeStats);
				fetchPipelineData(selectedPipeline);
			} else {
				fetchPipelineData(selectedPipeline);
			}
		} catch (error) {
			console.error('Error fetching running pipelines:', error);
			alert(`Failed to fetch running pipelines: ${error.message || error}`);
		}
	});

	let products: IndexingStatistics;

	const indexingStatisticsTemplate: IndexingStatistics = {
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
	};

	let productsArray = convertStatsToArray(indexingStatisticsTemplate);

	async function fetchPipelineData(selectedPipeline: string) {
		try {
			if (!selectedPipeline || selectedPipeline == 'no_active_pipeline') {
				isLive = false;
				return;
			}
			const response = await commands.describePipeline(selectedPipeline);
			if (response.status === 'ok') {
				products = response.data;
				describeStats.set(products);
				productsArray = convertStatsToArray(products);
				isLive = true;
			} else {
				isLive = false;
				throw new Error(`Unexpected response status: ${response.status}`);
			}
		} catch (error) {
			isLive = false;
			console.error('Error fetching pipeline data:', error);
		}
	}

	let intervalId: ReturnType<typeof setInterval>;
	onMount(() => {
		fetchPipelineData(selectedPipeline);
		intervalId = setInterval(() => {
			fetchPipelineData(selectedPipeline);
		}, 5000);
	});

	onDestroy(() => {
		clearInterval(intervalId);
	});

	function convertStatsToArray(products: IndexingStatistics) {
		return Object.entries(products).map(([key, value]) => ({
			label: key.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase()),
			number: value
		}));
	}
</script>

<Card size="xl">
	<div class="mb-4 flex items-center gap-2">
		<h2 class="text-center text-[24px] font-semibold text-gray-900 dark:text-white">
			{'Data Fabric Pipeline Stats'}
			{#if isLive}
				<span class="blinking-dot"></span>
			{/if}
		</h2>
	</div>

	<!-- Dropdown to select pipeline -->
	<div class="mb-4">
		<label
			for="pipelineSelect"
			class="mb-2 block text-sm font-medium text-gray-700 dark:text-white"
		>
			Select Pipeline
		</label>
		<select
			id="pipelineSelect"
			class="block w-full rounded-lg border border-gray-300 bg-gray-50 p-2.5 text-sm text-gray-900 focus:border-blue-500 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400 dark:focus:border-blue-500 dark:focus:ring-blue-500"
			bind:value={selectedPipeline}
			on:change={() => fetchPipelineData(selectedPipeline)}
		>
			{#each runningPipelines as pipeline}
				<option value={pipeline}>{pipeline}</option>
			{/each}
			{#if runningPipelines.length === 0}
				<option value="no_active_pipeline">No Active Pipeline</option>
			{/if}
		</select>
	</div>

	<!-- Display stats in a grid layout -->
	<div class="grid grid-cols-1 gap-2 md:grid-cols-2">
		{#each productsArray as { label, number }}
			<Card class="rounded-lg bg-gray-50 p-4 shadow-md dark:bg-gray-800">
				<p class="font-medium text-gray-900 dark:text-white">
					{label}
				</p>
				<p class="text-lg font-semibold text-gray-900 dark:text-white">
					{number}
				</p>
			</Card>
		{/each}
	</div>
</Card>

<!-- Add the style block for pulsating dot directly in the component -->
<style>
	@keyframes blink {
		0% {
			opacity: 1;
		}
		50% {
			opacity: 0;
		}
		100% {
			opacity: 1;
		}
	}

	.blinking-dot {
		display: inline-block;
		width: 12px;
		height: 12px;
		margin-left: 10px;
		background-color: #ff6384;
		border-radius: 50%;
		animation: blink 1.5s infinite;
	}
</style>
