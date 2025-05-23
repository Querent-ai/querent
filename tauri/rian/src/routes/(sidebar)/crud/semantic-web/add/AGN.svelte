<script lang="ts">
	import Modal from '../../sources/add/Modal.svelte';
	import {
		addPipelinesToList,
		dataSources,
		pipelineStartTime,
		pipelineState,
		updatePipeline,
		type PipelinesData
	} from '../../../../../stores/appState';

	import { Search } from 'flowbite-svelte';
	import Huggingface from './Huggingface.svelte';
	import { createEventDispatcher, onMount, tick } from 'svelte';
	import {
		commands,
		type Backend,
		type FixedEntities,
		type SampleEntities,
		type SemanticPipelineRequest
	} from '../../../../../service/bindings';
	import { goto } from '$app/navigation';
	export let formOpen: boolean;
	import { open, save } from '@tauri-apps/plugin-dialog';
	import { writeTextFile } from '@tauri-apps/plugin-fs';
	import { writable } from 'svelte/store';
	import LoadingModal from '../../discovery/LoadingModal.svelte';
	const isLoading = writable(false);

	let sourceIds: string[] = [];
	let sourceNames: string[] = [];

	let selectedSourceIds: string[] = [];
	let selectedModel: number | null = null;
	let isDropdownOpen = false;

	onMount(async () => {
		try {
			if ($dataSources) {
				let sources = await commands.getCollectors();
				sources.config.forEach((metadata) => {
					sourceIds = [...sourceIds, getId(metadata.backend)];
					sourceNames = [...sourceNames, metadata.name];
				});
			}
		} catch (error) {
			console.error('Error during onMount:', error);
			showModal = true;
			modalMessage = 'Failed to load data sources. Please try again.';
		}
	});

	function getId(backend: Backend | null): string {
		if (!backend) return '';
		if ('azure' in backend) return backend.azure.id;
		if ('drive' in backend) return backend.drive.id;
		if ('files' in backend) return backend.files.id;
		if ('dropbox' in backend) return backend.dropbox.id;
		if ('email' in backend) return backend.email.id;
		if ('gcs' in backend) return backend.gcs.id;
		if ('github' in backend) return backend.github.id;
		if ('jira' in backend) return backend.jira.id;
		if ('news' in backend) return backend.news.id;
		if ('onedrive' in backend) return backend.onedrive.id;
		if ('s3' in backend) return backend.s3.id;
		if ('slack' in backend) return backend.slack.id;
		return '';
	}

	let fixed_entities: FixedEntities = {
		entities: []
	};

	let sample_entities: SampleEntities = {
		entities: []
	};

	let request: SemanticPipelineRequest = {
		collectors: [],
		fixed_entities: fixed_entities,
		sample_entities: sample_entities,
		model: 0
	};

	let showModal = false;
	let modalMessage = '';

	let entityTable: {
		entity: string;
		entityType: string;
	}[] = [];

	let uploadedHeaders: string | any[] = [];
	let uploadedData: any[] = [];
	let fileInput: HTMLInputElement | null = null;

	let modelsList: { [key: string]: number } = {
		english: 0,
		geobert: 1
	};

	interface Model {
		id: number;
		value: string;
		name: string;
		info: string;
	}

	let newEntity = '';
	let newEntityType = '';
	let showNewEntityForm = false;
	let searchTerm = '';

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		try {
			const nonEmptyRows = entityTable.filter((row) => row.entity !== '' && row.entityType !== '');

			if ((selectedModel == null || selectedModel == -1) && nonEmptyRows.length == 0) {
				modalMessage =
					'Please define your search space by entering entities and their respective types';
				showModal = true;
				return;
			}

			let selectedSources: string[] = sourceIds.filter((_, index) => {
				return selectedSourceIds.includes(sourceNames[index]);
			});

			let request: SemanticPipelineRequest = {
				collectors: selectedSources,
				model: null,
				fixed_entities: {
					entities: nonEmptyRows.map((row) => row.entity)
				},
				sample_entities: {
					entities: nonEmptyRows.map((row) => row.entityType)
				}
			};

			isLoading.set(true);
			let result = await commands.startAgnFabric(request);

			if (result.status == 'ok') {
				updatePipeline('running', result.data.pipeline_id);
				let dateTime = Date.now();
				let dateTimeSeconds = Math.round(dateTime / 1000);
				pipelineStartTime.set(dateTimeSeconds);

				let pipelineMetadata: PipelinesData = {
					id: result.data.pipeline_id,
					sources: sourceNames,
					fixed_entities: request.fixed_entities?.entities,
					sample_entities: request.sample_entities?.entities,
					mode: 'active'
				};

				addPipelinesToList(pipelineMetadata);
			} else {
				console.error('Failed to start the pipeline:', result.error);
				showModal = true;
				modalMessage = 'Failed to start the pipeline. Please try again.';
			}

			selectedModel = null;

			goto('/crud/semantic-web');
		} catch (error) {
			console.error('Error while starting the pipeline:', error);
			showModal = true;
			modalMessage = 'An error occurred while starting the pipeline. Please try again.';
		} finally {
			isLoading.set(false);
		}
	};

	const handleRemoveSource = (id: string) => {
		selectedSourceIds = selectedSourceIds.filter((source) => source !== id);
	};

	const handleAddSource = (event: Event) => {
		const select: HTMLSelectElement = event.target as HTMLSelectElement;
		const selectedValue = select.value;
		if (selectedValue && !selectedSourceIds.includes(selectedValue)) {
			selectedSourceIds = [...selectedSourceIds, selectedValue];
		}
		select.value = '';
	};

	const toggleDropdown = (event: MouseEvent) => {
		event.preventDefault();
		event.stopPropagation();
		isDropdownOpen = !isDropdownOpen;
	};

	const dispatch = createEventDispatcher();

	const handleClose = () => {
		formOpen = false;
		dispatch('close');
	};

	function addRow() {
		showNewEntityForm = true;
	}

	function deleteRow(entityToDelete: { entity: string; entityType: string }) {
		const index = entityTable.findIndex(
			(item) =>
				item.entity === entityToDelete.entity && item.entityType === entityToDelete.entityType
		);
		if (index !== -1) {
			entityTable.splice(index, 1);
			entityTable = [...entityTable];
		}
	}

	function saveNewEntity(event: { preventDefault: () => void }) {
		event.preventDefault();
		if (newEntity && newEntityType) {
			entityTable = [...entityTable, { entity: newEntity, entityType: newEntityType }];
			newEntity = '';
			newEntityType = '';
			showNewEntityForm = false;
		} else {
			showModal = true;
			modalMessage = 'Please enter both entity and its type';
		}
	}
	let filteredTable: {
		entity: string;
		entityType: string;
	}[];

	$: filteredTable = entityTable.filter(
		(row) =>
			row.entity.toLowerCase().includes(searchTerm.toLowerCase()) ||
			row.entityType.toLowerCase().includes(searchTerm.toLowerCase())
	);

	async function downloadCSV() {
		let exampleCSV = 'Entity, Entity type';

		try {
			const filePath = await save({
				defaultPath: 'example.csv',
				filters: [
					{
						name: 'csv',
						extensions: ['csv']
					}
				]
			});

			if (filePath) {
				await writeTextFile(filePath, exampleCSV);
			}
		} catch (error) {
			console.error('Error saving file:', error);
			showModal = true;
			modalMessage = 'Failed to download CSV file. Please try again.';
		}
	}

	function handleFileUpload(event: Event) {
		try {
			event.preventDefault();
			const input = event.target as HTMLInputElement;
			if (input.files && input.files.length > 0) {
				const file = input.files[0];
				const reader = new FileReader();
				reader.onload = function (e) {
					const contents = e.target?.result;
					if (typeof contents === 'string') {
						const lines = contents?.split('\n');
						uploadedHeaders = lines[0].split(',');
						uploadedData = lines.slice(1).map((line: string) => line.split(','));
						let newEntities: { entity: any; entityType: any }[] = [];

						uploadedData.forEach((data) => {
							if (data.length === 2 && data[0].trim() !== '' && data[1].trim() !== '') {
								newEntities.push({
									entity: data[0].trim(),
									entityType: data[1].trim()
								});
							} else {
								console.log('Skipping invalid data:', data);
							}
						});

						if (newEntities.length > 0) {
							entityTable = [...entityTable, ...newEntities];
							entityTable = [...entityTable];
						}
						input.value = '';
					} else {
						throw new Error('Failed to read file as string');
					}
				};
				reader.readAsText(file);
			} else {
				input.value = '';
			}
		} catch (error) {
			console.error('Error uploading file:', error);
			showModal = true;
			modalMessage = 'An error occurred during file upload. Please try again.';
		}
	}

	function openFileInput() {
		if (fileInput) {
			fileInput.click();
		}
	}
</script>

{#if formOpen}
	<div class="form-container">
		<button class="close-button" on:click={handleClose}>&times;</button>
		<form on:submit|preventDefault={handleSubmit}>
			<div class="section">
				<div class="select-with-tags">
					<label for="sourceSelector"
						>Select Sources <span class="tooltip"
							><span class="icon-info">i</span><span class="tooltiptext"
								>Choose all the sources from the list of sources that you have defined</span
							>
						</span></label
					>
					<select id="sourceSelector" on:change={handleAddSource}>
						<option value="">-- Choose Sources --</option>
						{#each sourceNames as names}
							<option value={names}>{names}</option>
						{/each}
					</select>
					<div class="tags">
						{#each selectedSourceIds as id}
							<span class="tag">
								{id}
								<button type="button" on:click={() => handleRemoveSource(id)}>&times;</button>
							</span>
						{/each}
					</div>
				</div>
			</div>

			<div class="divider"></div>

			<div class="section">
				<label for="entity-pairs">
					Entity Search Space <span class="tooltip"
						><span class="icon-info">i</span>
						<span class="tooltiptext"
							>Enter the entity and its types in the below table or upload your CSV</span
						>
					</span>
				</label>
				<div class="search-container">
					<Search size="md" style="width: 300px" class="search-btn" bind:value={searchTerm} />
					<button type="button" class="add-row-btn" on:click={addRow}>+ Add New Entity Pair</button>
				</div>
				<div class="table-container">
					<table>
						<thead>
							<tr>
								<th>Entity</th>
								<th>Entity Type</th>
								<th>Actions</th>
							</tr>
						</thead>
						<tbody>
							{#each filteredTable as row, index (row.entity + row.entityType)}
								<tr>
									<td>
										{row.entity}
									</td>
									<td>
										{row.entityType}
									</td>
									<td class="button-cell">
										<button class="delete-button" on:click={() => deleteRow(row)}>Delete</button>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
				{#if showNewEntityForm}
					<div class="new-entity-form">
						<input type="text" bind:value={newEntity} placeholder="Entity" />
						<input type="text" bind:value={newEntityType} placeholder="Entity Type" />
						<button on:click={saveNewEntity}>Save</button>
					</div>
				{/if}

				<div class="button-container">
					<button type="button" class="open-csv-btn" on:click={openFileInput}
						>Upload your CSV</button
					>
					<button type="button" class="download-csv-btn" on:click={downloadCSV}>
						<span class="tooltip"
							><svg
								class="h-6 w-6 text-gray-800 dark:text-white"
								aria-hidden="true"
								xmlns="http://www.w3.org/2000/svg"
								width="24"
								height="24"
								fill="none"
								viewBox="0 0 24 24"
							>
								<path
									stroke="currentColor"
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M12 13V4M7 14H5a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-4a1 1 0 0 0-1-1h-2m-1-5-4 5-4-5m9 8h.01"
								/>
							</svg>

							<span class="tooltiptext">Download example CSV</span>
						</span>
					</button>
					<input
						type="file"
						accept=".csv"
						bind:this={fileInput}
						style="display: none;"
						on:change={handleFileUpload}
					/>
				</div>
			</div>

			<button type="submit">Start Pipeline</button>
		</form>

		<Modal bind:show={showModal} message={modalMessage} />

		{#if $isLoading}
			<LoadingModal message="Please wait while we get your data...." />
		{/if}
	</div>
{/if}

<style>
	.form-container {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 20px;
		border-radius: 8px;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
		background: #fff;
		width: 90%;
		max-width: 1000px;
		margin: 20px auto;
	}

	.form-container label {
		display: block;
		margin-bottom: 10px;
		font-size: 1.1em;
		font-weight: bold;
	}

	.select-with-tags select,
	.tags {
		width: 100%;
		max-width: 100%;
		padding: 10px;
		margin-bottom: 20px;
		border: 1px solid #ddd;
		border-radius: 4px;
		box-sizing: border-box;
	}

	.select-with-tags {
		display: flex;
		flex-direction: column;
		width: 100%;
		max-width: 100%;
		box-sizing: border-box;
		margin-bottom: 20px;
	}

	select {
		width: 100%;
		padding: 10px;
		border-radius: 4px;
		margin-bottom: 10px;
		box-sizing: border-box;
	}

	.custom-dropdown {
		position: relative;
		width: 100%;
		max-width: 100%;
	}

	.dropdown-toggle {
		width: 100%;
		padding: 10px;
		border-radius: 4px;
		margin-bottom: 10px;
		box-sizing: border-box;
		text-align: left;
		background-color: white;
		border: 1px solid #ddd;
		cursor: pointer;
	}

	.dropdown-menu {
		position: absolute;
		top: 100%;
		left: 0;
		width: 100%;
		background-color: white;
		border: 1px solid #ddd;
		border-radius: 4px;
		box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
		z-index: 10;
		max-height: 300px;
		overflow-y: auto;
	}

	.model-item {
		display: flex;
		align-items: center;
		padding: 10px;
		border-bottom: 1px solid #eee;
		cursor: pointer;
		width: 100%;
	}

	.model-details {
		flex: 1;
		text-align: center;
		padding-left: 100px;
	}

	.model-name {
		font-weight: bold;
		margin-bottom: 5px;
	}

	.model-description {
		font-size: 0.9em;
		color: #666;
	}
	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
		margin-bottom: 20px;
		max-height: 100px;
		overflow-y: auto;
		padding: 5px;
		width: 100%;
		align-items: center;
		border: 1px solid transparent;
	}

	.tag {
		padding: 5px 10px;
		background-color: #007bff;
		color: white;
		border-radius: 20px;
		display: inline-flex;
		align-items: center;
		margin: 2px;
	}

	.tag button {
		background: none;
		border: none;
		color: white;
		cursor: pointer;
		margin-left: 5px;
	}

	.close-button {
		position: absolute;
		top: 10px;
		right: 10px;
		border: none;
		background: #888;
		color: white;
		border-radius: 50%;
		width: 30px;
		height: 30px;
		font-weight: bold;
		cursor: pointer;
	}

	.close-button:hover {
		background: #555;
	}

	button[type='submit'] {
		padding: 10px 20px;
		background: #007bff;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		display: block;
		margin: 0 auto;
		border-radius: 20px;
		font-size: 1em;
		margin-bottom: 20px;
	}

	button[type='submit']:hover {
		background: #007bff;
	}

	.tooltip {
		position: relative;
		display: inline-block;
	}

	.icon-info {
		display: inline-block;
		width: 14px;
		height: 14px;
		vertical-align: 5px;
		border-radius: 50%;
		background-color: #898e92;
		color: black;
		text-align: center;
		line-height: 14px;
		font-size: 12px;
		font-weight: bold;
		font-style: italic;
		font-family: 'Franklin Gothic Medium', 'Arial Narrow', Arial, sans-serif;
		cursor: pointer;
	}

	.tooltip .tooltiptext {
		visibility: hidden;
		width: 300px;
		background-color: #007bff;
		color: #fff;
		text-align: center;
		border-radius: 6px;
		padding: 5px;
		position: absolute;
		z-index: 1;
		bottom: 125%;
		left: 50%;
		margin-left: -150px;
		opacity: 0;
		transition: opacity 0.3s;
		font-size: 12px;
		line-height: 1.2;
	}

	.tooltip:hover .tooltiptext {
		visibility: visible;
		opacity: 1;
	}
	.divider {
		height: 2px;
		background: #ccc;
		margin: 20px 0;
	}

	.section {
		margin-bottom: 20px;
		text-align: center;
	}

	.open-csv-btn {
		display: inline-block;
		margin: 0;
		background-color: #007bff;
		color: white;
		border: none;
		padding: 5px 20px;
		border-radius: 20px;
		cursor: pointer;
		font-size: 1em;
		margin-bottom: 26px;
		border-radius: 20px;
		white-space: nowrap;
	}

	.download-csv-btn {
		display: inline-block;
		margin: 0;
		background-color: white;
		color: white;
		border: none;
		padding: 10px 20px;
		border-radius: 20px;
		cursor: pointer;
		font-size: 1em;
		margin-bottom: 20px;
		border-radius: 20px;
		white-space: nowrap;
	}

	.add-row-btn {
		display: block;
		margin: 0 auto;
		background-color: #007bff;
		color: white;
		border: none;
		padding: 10px 20px;
		border-radius: 20px;
		cursor: pointer;
		font-size: 1em;
		white-space: nowrap;
	}

	.table-container {
		display: flex;
		flex-direction: column;
		height: auto;
		overflow-y: auto;
		margin-bottom: 10px;
		border: 1px solid #ddd;
		width: 100%;
	}

	table {
		width: 100%;
		border-collapse: collapse;
		display: grid;
		min-width: 900px;
	}

	thead {
		position: sticky;
		top: 0;
		background-color: #c0bdbd;
		color: white;
		z-index: 1;
		display: contents;
	}
	tbody {
		width: 100%;
		min-width: 600px;
	}

	td {
		padding: 8px 10px;
		text-align: left;
		border: none;
		width: 50%;
	}

	.button-cell {
		display: flex;
		justify-content: flex-start;
		align-items: center;
	}

	th {
		color: black;
		text-transform: uppercase;
		font-size: 1em;
		border-bottom: 2px solid #0c0c0c;
		width: 50%;
		font-weight: normal;
		padding: 8px 10px;
		text-align: left;
	}

	tr {
		border-bottom: 1px solid #3a4453;
		width: 100%;
		min-width: 600px;
	}

	tr:last-child {
		border-bottom: none;
		width: 100%;
		min-width: 600px;
	}

	.delete-button {
		cursor: pointer;
		margin-left: 5px;
		background-color: #007bff;
		color: white;
		padding: 5px 5px;
		font-size: 0.9em;
		border: none;
		border-radius: 10px;
		display: inline-block;
	}

	.search-container {
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: #eeecec;
		height: 60px;
		padding: 10px;
	}

	.button-container {
		display: flex;
		justify-content: flex-end;
		gap: 5px;
		margin-bottom: 20px;
	}

	.new-entity-form {
		display: grid;
		grid-template-columns: 1fr 1fr 80px;
		gap: 1px;
		background-color: #ffffff;
		border: 1px solid #ddd;
		border-top: none;
	}
	.new-entity-form input,
	.new-entity-form button {
		padding: 8px;
		border: none;
		background-color: white;
	}

	.new-entity-form button {
		cursor: pointer;
		margin-left: 15px;
		margin-right: 10px;
		margin-bottom: 10px;
		background-color: #007bff;
		color: white;
		padding: 5px 5px;
		font-size: 0.9em;
		border: none;
		border-radius: 10px;
		display: inline-block;
	}
</style>
