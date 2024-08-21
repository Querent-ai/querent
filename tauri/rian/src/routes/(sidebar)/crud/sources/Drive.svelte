<script lang="ts">
	import { onMount } from 'svelte';
	import { Button, Input, Label } from 'flowbite-svelte';
	import { addDataSource, type CollectorMetadata } from '../../../../stores/appState';
	import { goto } from '$app/navigation';
	import { isVisible } from '../../../../stores/appState';
	import {
		commands,
		type CollectorConfig,
		type GoogleDriveCollectorConfig
	} from '../../../../service/bindings';

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
	let drive_client_id: string;
	let drive_client_secret: string;

	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		const url = new URL(window.location.href);
		const code = params.get('code');
		if (code) {
			try {
				const result = await commands.getDriveCredentials();
				if (result.status == 'ok') {
					drive_client_id = result.data[0];
					drive_client_secret = result.data[1];
				}
				const response = await fetch('https://oauth2.googleapis.com/token', {
					method: 'POST',
					headers: {
						'Content-Type': 'application/x-www-form-urlencoded'
					},
					body: new URLSearchParams({
						code: code,
						client_id: drive_client_id,
						client_secret: drive_client_secret,
						redirect_uri: import.meta.env.VITE_DRIVE_REDIRECT_URL,
						grant_type: 'authorization_code'
					})
				});

				if (!response.ok) {
					console.error('HTTP error! status:', response.status);
					console.log('Error response body:', await response.text());
					return;
				}

				const data = await response.json();
				drive_config.drive_refresh_token = data.refresh_token;

				url.searchParams.delete('code');
				url.searchParams.delete('scope');
				window.history.replaceState({}, '', url.toString());
			} catch (error) {
				console.error('Error during token exchange:', error);
			}
		}
	});

	function handleSubmit() {
		if (folderPath && name) {
			drive_config.folder_to_crawl = folderPath;
			drive_config.id = crypto.randomUUID();
			collector_config.backend = { drive: drive_config };
			collector_config.name = name;

			metadata.id = drive_config.id;
			metadata.name = name;
			metadata.type = 'drive';
			addDataSource(collector_config);
			commands.setCollectors([collector_config]);

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
{/if}
