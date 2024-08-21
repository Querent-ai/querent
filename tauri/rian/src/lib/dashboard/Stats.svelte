<script lang="ts">
	import { Card, Heading, TabItem, Tabs } from 'flowbite-svelte';
	import { pipelineState } from '../../stores/appState';
	import { commands, type IndexingStatistics } from '../../service/bindings';
	import { onDestroy, onMount } from 'svelte';

	let selectedPipeline: string;

	$: selectedPipeline = $pipelineState?.id || '123456';

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
		const response = await commands.describePipeline(selectedPipeline);
		if (response.status == 'ok') {
			products = response.data;
			productsArray = convertStatsToArray(products);
		}
	}

	let intervalId: ReturnType<typeof setInterval>;

	onMount(() => {
		fetchPipelineData(selectedPipeline);
		intervalId = setInterval(() => {
			fetchPipelineData(selectedPipeline);
		}, 10000);
	});

	onDestroy(() => {
		clearInterval(intervalId);
	});

	function convertStatsToArray(products: IndexingStatistics) {
		return Object.entries(products).map(([key, value]) => ({
			label: key.replace(/_/g, ' ').replace(/\b\w/g, (l) => l.toUpperCase()), // Convert snake_case to Title Case
			number: value
		}));
	}
</script>

<Card size="xl">
	<div class="mb-4 flex items-center gap-2">
		<Heading tag="h3" class="w-fit text-lg font-semibold dark:text-white">Pipeline Stats</Heading>
	</div>
	<Tabs
		style="full"
		defaultClass="flex divide-x rtl:divide-x-reverse divide-gray-200 shadow dark:divide-gray-700"
		contentClass="p-3 mt-4"
	>
		<TabItem class="w-full" open>
			<select
				slot="title"
				id="pipelineSelect"
				class="block w-full rounded-lg border border-gray-300 bg-gray-50 p-2.5 text-sm text-gray-900 focus:border-blue-500 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400 dark:focus:border-blue-500 dark:focus:ring-blue-500"
			>
				<option value={selectedPipeline} selected>{selectedPipeline}</option>
			</select>
			<ul class="-m-3 divide-y divide-gray-200 dark:divide-gray-700 dark:bg-gray-800">
				{#each productsArray as { label, number }}
					<li class="py-3 sm:py-4">
						<div class="flex items-center justify-between">
							<div class="flex min-w-0 items-center">
								<div class="ml-3">
									<p class="truncate font-medium text-gray-900 dark:text-white">
										{label}
									</p>
								</div>
							</div>
							<div
								class="inline-flex items-center text-base font-semibold text-gray-900 dark:text-white"
							>
								{number}
							</div>
						</div>
					</li>
				{/each}
			</ul>
		</TabItem>
	</Tabs>

	<div
		class="mt-4 flex items-center justify-between border-t border-gray-200 pt-3 dark:border-gray-700 sm:pt-6"
	></div>
</Card>
