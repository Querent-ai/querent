<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Input, Label } from 'flowbite-svelte';
	import {
		addDataSource,
		googleDriveRefreshToken,
		type CollectorMetadata
	} from '../../../../stores/appState';
	import { goto } from '$app/navigation';
	import { isVisible } from '../../../../stores/appState';
	import {
		commands,
		type CollectorConfig,
		type GoogleDriveCollectorConfig
	} from '../../../../service/bindings';
	import Modal from './add/Modal.svelte';

	let drive_config: GoogleDriveCollectorConfig = {
		drive_client_id: import.meta.env.VITE_DRIVE_CLIENT_ID,
		drive_client_secret: import.meta.env.VITE_DRIVE_CLIENT_SECRET,
		drive_refresh_token: '',
		folder_to_crawl: '',
		id: ''
	};

	let metadata: CollectorMetadata = {
		id: '',
		name: '',
		type: ''
	};

	function handleClose() {
		$isVisible = false;
	}
	let collector_config: CollectorConfig = {
		name: '',
		backend: {
			drive: drive_config as GoogleDriveCollectorConfig
		}
	};

	let folderPath = '';
	let name = '';
	let showModal = false;
	let modalMessage = '';

	async function handleSubmit() {
		if (folderPath && name) {
			if (!$googleDriveRefreshToken || $googleDriveRefreshToken == '') {
				modalMessage = 'Please complete the oauth first ';
				showModal = true;
				return;
			}
			drive_config.folder_to_crawl = folderPath;
			drive_config.id = crypto.randomUUID();
			drive_config.drive_refresh_token = $googleDriveRefreshToken;

			collector_config.backend = { drive: drive_config };
			collector_config.name = name;

			metadata.id = drive_config.id;
			metadata.name = name;
			metadata.type = 'drive';
			addDataSource(collector_config);
			commands.setCollectors([collector_config]);

			googleDriveRefreshToken.set('');

			goto('/crud/sources');
		} else {
			console.error('Please fill in all required fields.');
		}
	}
</script>

{#if $isVisible}
	<div class="flex min-h-screen items-start justify-center pt-20">
		<form
			on:submit|preventDefault={handleSubmit}
			class="relative rounded-lg bg-white p-4 shadow-lg"
		>
			<button
				type="button"
				class="absolute right-0 top-0 m-4 text-lg font-bold text-gray-800 hover:text-gray-600"
				on:click={handleClose}
				aria-label="Close form">&times;</button
			>
			<Label class="mb-5 block w-full space-y-2">
				<span>Name</span>
				<Input
					bind:value={name}
					class="border font-normal outline-none"
					placeholder="Enter the name for the source"
					required
					style="min-width: 300px;"
				/>
			</Label>

			<Label class="mb-5 block w-full space-y-2">
				<span>Folder to crawl</span>
				<Input
					bind:value={folderPath}
					class="border font-normal outline-none"
					placeholder="Enter path of your folder to crawl"
					required
					style="min-width: 300px;"
				/>
			</Label>

			<div class="flex w-full pb-5">
				<Button type="submit" class="w-full rounded bg-blue-600 px-4 py-2 text-base text-white"
					>Save Configuration</Button
				>
			</div>
		</form>
	</div>

	<Modal bind:show={showModal} message={modalMessage} />
{/if}
