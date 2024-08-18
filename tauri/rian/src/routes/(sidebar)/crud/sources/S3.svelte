<script lang="ts">
	import { Button, Heading, Input, Label } from 'flowbite-svelte';

	import { createEventDispatcher, onMount } from 'svelte';
	const dispatch = createEventDispatcher();

	export let sourceName: string = 'AWS S3';

	let bucket: string = '';
	let region: string = '';
	let access_key: string = '';
	let secret_key: string = '';

	export let configuration: {
		Bucket: string;
		Region: string;
		'Access Key': string;
		'Secret Key': string;
	};
	function initializeForm() {
		if (configuration) {
			bucket = configuration['Bucket'] || '';
			region = configuration['Region'] || '';
			access_key = configuration['Access Key'] || '';
			secret_key = configuration['Secret Key'] || '';
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
			technology: 'AWS S3',
			description: 'Configured S3 bucket',
			configuration: {
				Bucket: bucket,
				Region: region,
				'Access Key': access_key,
				'Secret Key': secret_key
			}
		});
	}
</script>

<Heading tag="h5" class="mb-6 text-sm font-semibold uppercase">Configure S3 Source</Heading>

<form on:submit|preventDefault={saveConfiguration}>
	<div class="space-y-4">
		<Label class="space-y-2">
			<span>Bucket</span>
			<Input
				bind:value={bucket}
				class="border font-normal outline-none"
				placeholder="Type bucket name"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Region</span>
			<Input
				bind:value={region}
				class="border font-normal outline-none"
				placeholder="Type region of your bucket"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Access Key</span>
			<Input
				bind:value={access_key}
				class="border font-normal outline-none"
				placeholder="Type access key"
				required
			/>
		</Label>
		<Label class="space-y-2">
			<span>Secret Key</span>
			<Input
				bind:value={secret_key}
				class="border font-normal outline-none"
				placeholder="Type secret_key"
				required
			/>
		</Label>

		<div class="flex w-full justify-center space-x-4 pb-4">
			<Button type="submit" class="w-full">Save Configuration</Button>
		</div>
	</div>
</form>
