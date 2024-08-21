<script lang="ts">
	import { Button, Heading, Input, Label } from 'flowbite-svelte';
	import { createEventDispatcher, onMount } from 'svelte';
	const dispatch = createEventDispatcher();

	export let sourceName: string = 'Dropbox';

	let dropbox_app_key: string = '';
	let dropbox_refresh_token: string = '';
	let dropbox_app_secret: string = '';
	let folder_path: string = '';

	export let configuration: {
		'Dropbox App Key': string;
		'Dropbox Refresh Token': string;
		'Dropbox App Secret': string;
		'Dropbox Folder Path': string;
	};
	function initializeForm() {
		if (configuration) {
			dropbox_app_key = configuration['Dropbox App Key'] || '';
			dropbox_refresh_token = configuration['Dropbox Refresh Token'] || '';
			dropbox_app_secret = configuration['Dropbox App Secret'] || '';
			folder_path = configuration['Dropbox Folder Path'] || '';
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
			technology: 'Dropbox',
			description: 'Configured Dropbox',
			configuration: {
				'Dropbox App Key': dropbox_app_key,
				'Dropbox Refresh Token': dropbox_refresh_token,
				'Dropbox App Secret': dropbox_app_secret,
				'Dropbox Folder Path': folder_path
			}
		});
	}
</script>

<Heading tag="h5" class="mb-6 text-sm font-semibold uppercase">Configure Dropbox</Heading>

<form on:submit|preventDefault={saveConfiguration}>
	<div class="space-y-4">
		<Label class="space-y-2">
			<span>Dropbox App Key</span>
			<Input
				bind:value={dropbox_app_key}
				class="border font-normal outline-none"
				placeholder="Enter your Dropbox App Key"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Dropbox Refresh Token</span>
			<Input
				bind:value={dropbox_refresh_token}
				class="border font-normal outline-none"
				placeholder="Enter your Dropbox Refresh Token"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Dropbox App Secret</span>
			<Input
				bind:value={dropbox_app_secret}
				class="border font-normal outline-none"
				placeholder="Enter your Dropbox App Secret"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Dropbox Folder Path</span>
			<Input
				bind:value={folder_path}
				class="border font-normal outline-none"
				placeholder="Enter Dropbox Folder Path"
				required
			/>
		</Label>

		<div class="flex w-full justify-center space-x-4 pb-4">
			<Button type="submit" class="w-full">Save Configuration</Button>
		</div>
	</div>
</form>
