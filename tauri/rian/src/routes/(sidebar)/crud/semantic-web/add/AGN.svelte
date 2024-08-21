<script lang="ts">
	import Modal from '../../sources/add/Modal.svelte';
	import {
		addPipelinesToList,
		dataSources,
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

	let sourceIds: string[] = [];
	let sourceNames: string[] = [];

	let selectedSourceIds: string[] = [];
	let selectedModel: number | null = null;
	let isDropdownOpen = false;

	onMount(async () => {
		if ($dataSources) {
			let sources = await commands.getCollectors();
			sources.config.forEach((metadata) => {
				sourceIds = [...sourceIds, getId(metadata.backend)];
				sourceNames = [...sourceNames, metadata.name];
			});
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
	// let entitiesPairs: any[] = [];
	// let showEntityModal = false;
	let entityTable: {
		editing: boolean;
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

	let models = [
		{
			id: 0,
			value: 'english',
			name: 'Davlan/xlm-roberta-base-wikiann-ner',
			info: 'First Named Entity Recognition model for 20 languages based on a fine-tuned XLM-RoBERTa large model'
		},
		{
			id: 1,
			value: 'geobert',
			name: 'botryan96/GeoBERT',
			info: 'An NER model that was fine-tuned from SciBERT on the Geoscientific Corpus dataset. The model was trained on the Labeled Geoscientific Corpus dataset'
		}
	];

	$: selectedModelName =
		selectedModel !== null
			? models.find((m) => m.id === selectedModel)?.name
			: '-- Choose Model --';

	const handleSubmit = async (event: Event) => {
		event.preventDefault();
		console.log('table   ', entityTable);
		const nonEmptyRows = entityTable.filter((row) => row.entity !== '' && row.entityType !== '');

		console.log('ROws are ', nonEmptyRows.length);
		console.log('Selected model  ', selectedModel);

		if ((selectedModel == null || selectedModel == -1) && nonEmptyRows.length == 0) {
			modalMessage = 'Please either choose model or enter some entities';
			showModal = true;
			return;
		}


		let selectedSources: string[] = sourceIds.filter((_, index) => {
			return selectedSourceIds.includes(sourceNames[index]);
		});

		let request: SemanticPipelineRequest = {
			collectors: selectedSources,
			model: selectedModel,
			fixed_entities: {
				entities: nonEmptyRows.map((row) => row.entity)
			},
			sample_entities: {
				entities: nonEmptyRows.map((row) => row.entityType)
			}
		};
		console.log("This is the semantic pipeline request ::::", request);
		let result = await commands.startAgnFabric(request);

		if (result.status == 'ok') {
			console.log('Pipeline id is ', result.data.pipeline_id);
			updatePipeline('running', result.data.pipeline_id);

			let pipelineMetadata: PipelinesData = {
				id: result.data.pipeline_id,
				sources: sourceNames,
				fixed_entities: request.fixed_entities?.entities,
				sample_entities: request.sample_entities?.entities
			};

			addPipelinesToList(pipelineMetadata);
		}

		selectedModel = null;

		goto('/crud/semantic-web');
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

	const handleAddModel = (model: Model) => {
		if (model.value === 'geobert') {
			modalMessage = 'This feature is only available in the premium version.';
			showModal = true;
			selectedModel = null;
		} else if (model.value === 'english') {
			console.log('English is selected');
			selectedModel = modelsList[model.value];
		}
		isDropdownOpen = false;
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
		entityTable = [...entityTable, { entity: '', entityType: '', editing: true }];
		updateFocus(entityTable.length - 1);
	}

	function editRow(index: number) {
		entityTable[index].editing = true;
		updateFocus(index);
	}

	function saveRow(index: number) {
		entityTable[index].editing = false;
	}

	function deleteRow(index: number) {
		entityTable.splice(index, 1);
		entityTable = [...entityTable];
	}

	async function updateFocus(index: number) {
		await tick();
		const input = document.getElementById(`entity-input-${index}`);
		if (input) {
			input.focus();
		}
	}

	function downloadCSV() {
		let exampleCSV = [['Entity', 'Entity type']];

		let csvContent = 'data:text/csv;charset=utf-8,' + exampleCSV.map((e) => e.join(',')).join('\n');

		let encodedUri = encodeURI(csvContent);

		let link = document.createElement('a');
		link.setAttribute('href', encodedUri);
		link.setAttribute('download', 'example.csv');
		document.body.appendChild(link); // Required for Firefox

		link.click();
		document.body.removeChild(link);
	}

	function handleFileUpload(event: Event) {
		event.preventDefault();
		console.log('Handling file input');
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
					let newEntities: { editing: boolean; entity: any; entityType: any }[] = [];

					uploadedData.forEach((data) => {
						if (data.length === 2 && data[0].trim() !== '' && data[1].trim() !== '') {
							newEntities.push({
								editing: false,
								entity: data[0].trim(),
								entityType: data[1].trim()
							});
						} else {
							console.log('Skipping invalid data:', data);
						}
					});
					console.log('New entities are ', newEntities.length);

					if (newEntities.length > 0) {
						entityTable = [...entityTable, ...newEntities];
						entityTable = [...entityTable];
					}
					input.value = '';
				} else {
					console.error('Failed to read file as string');
				}
			};
			reader.readAsText(file);
		} else {
			input.value = '';
		}
	}

	function openFileInput() {
		if (fileInput) {
			console.log('File input clicked ');
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
						>Select Source <span class="tooltip"
							><span class="icon-info">i</span><span class="tooltiptext"
								>Choose all the sources from the list of sources that you have defined</span
							>
						</span></label
					>
					<select id="sourceSelector" on:change={handleAddSource}>
						<option value="">-- Choose Source --</option>
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
					<Search size="md" style="width: 300px" class="search-btn" />
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
							{#each entityTable as row, index (row.entity + row.entityType)}
								<tr>
									<td>
										{#if row.editing}
											<input
												id={`entity-input-${index}`}
												class="input-field"
												type="text"
												bind:value={row.entity}
											/>
										{:else}
											{row.entity}
										{/if}
									</td>
									<td>
										{#if row.editing}
											<input
												id={`entity-input-${index}`}
												class="input-field"
												type="text"
												bind:value={row.entityType}
											/>
										{:else}
											{row.entityType}
										{/if}
									</td>
									<td class="button-cell">
										{#if row.editing}
											<button class="save-button" on:click={() => saveRow(index)}>Save</button>
										{:else}
											<button class="edit-button" on:click={() => editRow(index)}>Edit</button>
											<button class="delete-button" on:click={() => deleteRow(index)}>Delete</button
											>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

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

			<div class="divider"></div>

			<div class="section">
				<label for="model">
					Model
					<span class="tooltip">
						<span class="icon-info">i</span>
						<span class="tooltiptext">Choose the model to use for Named Entity Recognition.</span>
					</span>
				</label>

				<div class="custom-dropdown">
					<button on:click={toggleDropdown} class="dropdown-toggle">
						{selectedModelName}
					</button>

					{#if isDropdownOpen}
						<div class="dropdown-menu">
							{#each models as model}
								<button type="button" class="model-item" on:click={() => handleAddModel(model)}>
									<Huggingface height="30px" width="30px" />
									<div class="model-details">
										<div class="model-name">{model.name}</div>
										<div class="model-description">{model.info}</div>
									</div>
								</button>
							{/each}
						</div>
					{/if}
				</div>
			</div>

			<button type="submit">Start Pipeline</button>
		</form>

		<Modal bind:show={showModal} message={modalMessage} />
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

	.form-container input[type='text'],
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

	.section input[type='text'] {
		margin-bottom: 10px;
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

	td input {
		width: 90%;
		min-width: 200px;
		box-sizing: border-box;
		margin: 0;
		padding: 10px;
		border: 1px solid #ddd;
		background: transparent;
		font-size: 1em;
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

	.save-button {
		cursor: pointer;
		margin-left: 5px;
		background-color: #007bff;
		color: white;
		padding: 5px 10px;
		font-size: 0.9em;
		border: none;
		border-radius: 10px;
	}

	.edit-button,
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
	.input-field {
		width: 100%;
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
</style>
