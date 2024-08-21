<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import { Button, Heading, Input, Label, Textarea } from 'flowbite-svelte';

	const dispatch = createEventDispatcher();

	export let sourceName: string = 'Google Cloud Storage';
	export let configuration: { 'Bucket Name': string; Credentials: string };

	let bucketName = '';
	let credentials = '';

	function initializeForm() {
		if (configuration) {
			bucketName = configuration['Bucket Name'] || '';
			credentials = configuration['Credentials'] || '';
		}
	}

	onMount(() => {
		initializeForm();
	});

	$: if (configuration) {
		initializeForm();
	}

	function saveConfiguration() {
		dispatch('sourceSaved', {
			name: sourceName,
			technology: 'Google Cloud Storage',
			description: 'Configured GCS bucket: ' + bucketName,
			configuration: {
				'Bucket Name': bucketName,
				Credentials: credentials
			}
		});
	}
</script>

<Heading tag="h5" class="mb-6 text-sm font-semibold uppercase"
	>Configure Google Cloud Storage Bucket</Heading
>

<form on:submit|preventDefault={saveConfiguration}>
	<div class="space-y-4">
		<Label class="space-y-2">
			<span>Bucket Name</span>
			<Input
				id="bucket"
				bind:value={bucketName}
				class="border font-normal outline-none"
				placeholder="Enter your storage bucket name"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Credentials (Stringified JSON)</span>
			<Textarea
				id="credentials"
				bind:value={credentials}
				class="border font-normal outline-none"
				placeholder="Paste your JSON credentials here"
				rows="6"
				required
			></Textarea>
		</Label>

		<div class="flex w-full justify-center space-x-4 pb-4">
			<Button type="submit" class="w-full">Save Configuration</Button>
		</div>
	</div>
</form>
