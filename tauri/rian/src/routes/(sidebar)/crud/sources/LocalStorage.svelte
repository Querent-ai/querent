<script lang="ts">
	import { Button, Input, Label } from 'flowbite-svelte';
	import {
		commands,
		type CollectorConfig,
		type FileCollectorConfig
	} from '../../../../service/bindings';
	import { goto } from '$app/navigation';
	import { addDataSource, isVisible } from '../../../../stores/appState';
	// Import the Tauri fs plugin
	import { open } from '@tauri-apps/plugin-dialog';
	import ErrorModal from '$lib/dashboard/ErrorModal.svelte';
	let showErrorModal = false;
	let errorMessage = '';
	function closeErrorModal() {
		showErrorModal = false;
	}

	let file_collector_config: FileCollectorConfig = {
		root_path: '',
		id: ''
	};

	function handleClose() {
		$isVisible = false;
	}

	let collector_config: CollectorConfig = {
		name: '',
		backend: { files: file_collector_config }
	};

	let root_path = '';
	let name = '';

	async function selectDirectory() {
		const selected = await open({
			directory: true,
			multiple: false,
			title: 'Select Folder'
		});

		if (selected) {
			root_path = selected as string;
		} else {
			errorMessage = 'No directory selected.';
			showErrorModal = true;
		}
	}

	function updateDirectoryPath() {
		if (name && root_path) {
			file_collector_config.id = crypto.randomUUID();
			file_collector_config.root_path = root_path;
			collector_config.backend = { files: file_collector_config };
			collector_config.name = name;

			commands.setCollectors([collector_config]);

			addDataSource(collector_config);

			goto('/crud/sources');
		} else {
			console.log('No directory path or id entered.');
		}
	}
</script>

{#if $isVisible}
	<div class="flex min-h-screen items-start justify-center pt-20">
		<form
			on:submit|preventDefault={updateDirectoryPath}
			class="relative rounded-lg bg-white p-4 shadow-lg"
		>
			<button
				type="button"
				class="absolute right-0 top-0 m-4 text-lg font-bold text-gray-800 hover:text-gray-600"
				on:click={handleClose}
				aria-label="Close form">&times;</button
			>

			<Label class="mb-5 block w-full space-y-2">
				<span>Directory Path:</span>
				<div class="flex">
					<Input
						bind:value={root_path}
						class="border font-normal outline-none"
						placeholder="Select the directory"
						required
						style="min-width: 300px;"
						readonly
					/>
					<Button on:click={selectDirectory} class="ml-2">Browse</Button>
				</div>
			</Label>

			<Label class="mb-5 block w-full space-y-2">
				<span>Name</span>
				<Input
					bind:value={name}
					class="border font-normal outline-none"
					placeholder="Enter name for the source"
					required
					style="min-width: 300px;"
				/>
			</Label>

			<div class="flex w-full pb-5">
				<Button type="submit" class="w-full rounded bg-blue-600 px-4 py-2 text-base text-white">
					Save Configuration
				</Button>
			</div>
		</form>
	</div>
{/if}

{#if showErrorModal}
	<ErrorModal {errorMessage} closeModal={closeErrorModal} />
{/if}
