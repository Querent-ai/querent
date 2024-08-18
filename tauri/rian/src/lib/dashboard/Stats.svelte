<script lang="ts">
	import { Card, Heading, TabItem, Tabs } from 'flowbite-svelte';
	import { pipelineState } from '../../stores/appState';
	// import Change from '$lib/dashboard/Change.svelte';
	// import Customers from '../../routes/data/users.json';
	// import { imagesPath } from '$lib/variables';
	// import LastRange from '$lib/widgets/LastRange.svelte';
	// import More from '$lib/widgets/More.svelte';
	// import { QuestionCircleSolid } from 'flowbite-svelte-icons';

	let selectedPipeline: string;

	$: selectedPipeline = $pipelineState?.id || '123456';

	const products = [
		{
			label: 'Total Documents',
			number: 208
		},
		{
			label: 'Total Events',
			number: 1482
		},
		{
			label: 'Total Events Processed',
			number: 1482
		},
		{
			label: 'Total Events Received',
			number: 784
		},
		{
			label: 'Total Events Sent',
			number: 784
		},
		{
			label: 'Total Batches',
			number: 425
		},
		{
			label: 'Total Sentences',
			number: 742
		},
		{
			label: 'Total Subjects',
			number: 742
		},
		{
			label: 'Total Predicates',
			number: 742
		},
		{
			label: 'Total Objects',
			number: 742
		},
		{
			label: 'Total Graph Events',
			number: 742
		},
		{
			label: 'Total Vector Events',
			number: 742
		},
		{
			label: 'Total Data Processed Size',
			number: 1026
		}
	];

	// const customers = Customers.slice(0, 5);
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
				{#each products as { label, number }}
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
	>
		<!-- <LastRange />
		<More title="Full Report" href="#top" /> -->
	</div>
</Card>
