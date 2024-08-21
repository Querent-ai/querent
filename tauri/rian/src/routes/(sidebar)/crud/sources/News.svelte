<script lang="ts">
	import { Button, Heading, Input, Label, Select } from 'flowbite-svelte';

	import { createEventDispatcher, onMount } from 'svelte';
	const dispatch = createEventDispatcher();

	export let sourceName: string = 'News';

	let api_key: string = '';
	let query: string = '';
	let from_date: string = '';
	let to_date: string = '';
	let language: string = '';
	let sort_by: string = '';
	let page_size: string = '';
	let page: string = '';

	export let configuration: {
		'API Key': string;
		'Query Topic': string;
		'Date Range From': string;
		'Date Range To': string;
		Language: string;
		'Sort By': string;
		'Page Size': string;
		'Page Number': string;
	};
	function initializeForm() {
		if (configuration) {
			api_key = configuration['API Key'] || '';
			query = configuration['Query Topic'] || '';
			from_date = configuration['Date Range From'] || '';
			to_date = configuration['Date Range To'] || '';
			language = configuration['Language'] || '';
			sort_by = configuration['Sort By'] || '';
			page_size = configuration['Page Size'] || '';
			page = configuration['Page Number'] || '';
		}
	}

	onMount(() => {
		initializeForm();
	});

	$: if (configuration) {
		initializeForm();
	}

	function saveConfiguration() {
		dispatch('saveSource', {
			name: sourceName,
			technology: 'News',
			description: 'Configured News Source',
			configuration: {
				'API Key': api_key,
				'Query Topic': query,
				'Date Range From': from_date,
				'Date Range To': to_date,
				Language: language,
				'Sort By': sort_by,
				'Page Size': page_size,
				'Page Number': page
			}
		});
	}
</script>

<Heading tag="h5" class="mb-6 text-sm font-semibold uppercase">Configure News API</Heading>

<form on:submit|preventDefault={saveConfiguration}>
	<div class="space-y-4">
		<Label class="space-y-2">
			<span>API Key</span>
			<Input
				bind:value={api_key}
				type="password"
				class="border font-normal outline-none"
				placeholder="Enter your API key"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Query Topic</span>
			<Input
				bind:value={query}
				class="border font-normal outline-none"
				placeholder="Enter topic (e.g., technology)"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Date Range From</span>
			<Input bind:value={from_date} type="date" class="border font-normal outline-none" required />
		</Label>

		<Label class="space-y-2">
			<span>Date Range To</span>
			<Input bind:value={to_date} type="date" class="border font-normal outline-none" required />
		</Label>

		<Label class="space-y-2">
			<span>Language</span>
			<Select bind:value={language} class="border-gray-300 font-normal outline-none">
				<option value="en">English</option>
				<option value="es">Spanish</option>
				<option value="fr">French</option>
				<!-- Add other languages as needed -->
			</Select>
		</Label>

		<Label class="space-y-2">
			<span>Sort By</span>
			<Select bind:value={sort_by} class="border-gray-300 font-normal outline-none">
				<option value="publishedAt">Published At</option>
				<option value="relevancy">Relevancy</option>
				<option value="popularity">Popularity</option>
			</Select>
		</Label>

		<Label class="space-y-2">
			<span>Page Size</span>
			<Input
				bind:value={page_size}
				type="number"
				class="border font-normal outline-none"
				placeholder="Enter page size (e.g., 5)"
				min="1"
				max="100"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Page Number</span>
			<Input
				bind:value={page}
				type="number"
				class="border font-normal outline-none"
				placeholder="Enter page number (e.g., 1)"
				min="1"
				required
			/>
		</Label>

		<div class="flex w-full justify-center space-x-4 pb-4">
			<Button type="submit" class="w-full">Save Configuration</Button>
		</div>
	</div>
</form>
