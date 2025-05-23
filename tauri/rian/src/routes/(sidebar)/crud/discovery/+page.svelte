<script lang="ts">
	import Icon from '@iconify/svelte';
	import {
		discoveryApiResponseStore,
		discoverylist,
		discoveryPageNumber,
		discoveryQuery,
		discoverySessionId,
		firstDiscovery,
		type DiscoveryDataPageList
	} from '../../../../stores/appState';
	import { get, writable } from 'svelte/store';
	import { onMount, tick } from 'svelte';
	import { commands } from '../../../../service/bindings';
	import Modal from '../sources/add/Modal.svelte';
	import LoadingModal from './LoadingModal.svelte';
	import ErrorModal from '$lib/dashboard/ErrorModal.svelte';
	let showErrorModal = false;
	let errorMessage = '';
	function closeErrorModal() {
		showErrorModal = false;
	}

	let categoriesDropdown: string[] | null;
	let currentCategory: {
		sentence: string;
		document: string;
		source: string;
		relationship_strength: string;
		tags: string;
		top_pairs: string[];
	}[];
	const isLoading = writable(false);

	$: categories = $discoveryApiResponseStore.length > 0 ? $discoveryApiResponseStore : [];

	$: {
		currentCategory = categories.filter((cat) => cat.tags !== 'Filters');
		if (currentCategory.length > 0) {
			const top_pairs = currentCategory[0].top_pairs;
			categoriesDropdown = top_pairs;
		} else {
			categoriesDropdown = [];
		}

		let dropdown = categories.filter((cat) => cat.tags === 'Filters');
		if (dropdown.length > 0) {
			categoriesDropdown = dropdown[0].top_pairs;
			categoriesDropdown = [...categoriesDropdown, categories[0].top_pairs[0]];
			categoriesDropdown = [...categoriesDropdown, categories[1].top_pairs[0]];
		}
	}

	onMount(async () => {
		let discovery_data = get(discoverylist);

		if ((!discovery_data || discovery_data.length < 1) && get(firstDiscovery)) {
			console.log('Starting discovery session');
			isLoading.set(true);
			try {
				const res = await commands.sendDiscoveryRetrieverRequest('', []);

				if (res.status == 'ok') {
					discoverySessionId.set(res.data.session_id);
					let discoveryData: DiscoveryDataPageList = {
						page_number: res.data.page_ranking,
						data: res.data.insights
					};
					discoverylist.update((currentList) => {
						return [...currentList, discoveryData];
					});
					discoveryPageNumber.set(res.data.page_ranking);
					firstDiscovery.set(false);

					const insights = res.data.insights;

					if (insights) {
						categories = insights;
						discoveryApiResponseStore.set(insights);
					}
				} else {
					let error = res.error;
					if (typeof error === 'string' && error.startsWith('Error: ')) {
						error = error.replace('Error: ', '');
					}
					errorMessage = 'Unable to start discovery session' + error;
					showErrorModal = true;
				}
			} catch (error) {
				let err = error instanceof Error ? error.message : String(error);
				if (typeof err === 'string' && err.startsWith('Error: ')) {
					err = err.replace('Error: ', '');
				}
				errorMessage = 'Error starting discovery session: ' + err;
				showErrorModal = true;
			} finally {
				isLoading.set(false);
			}
		} else {
			const currentList = get(discoverylist);

			const matchingPage = currentList.find(
				(item) => item.page_number === get(discoveryPageNumber)
			);

			const insights = matchingPage?.data;

			if (insights) {
				categories = insights;
				discoveryApiResponseStore.set(insights);
			}
		}
	});

	let iconClass =
		'flex-shrink-0 w-9 h-9 text-gray-800 transition duration-75 group-hover:text-gray-900 dark:text-gray-400 dark:group-hover:text-white text-align:center';

	let icon = 'carbon:ibm-watson-discovery';

	let selectedCategories: any[] = [];

	async function toggleCategory(category: string) {
		isLoading.set(true);
		try {
			selectedCategories.push(category);
			const res = await commands.sendDiscoveryRetrieverRequest(query, selectedCategories);

			if (res.status == 'ok') {
				let discoveryData: DiscoveryDataPageList = {
					page_number: res.data.page_ranking,
					data: res.data.insights
				};
				discoverylist.update((currentList) => {
					return [...currentList, discoveryData];
				});
				const insights = res.data.insights;

				if (insights) {
					discoveryApiResponseStore.set(insights);
				}
			} else {
				let error = res.error;
				if (typeof error === 'string' && error.startsWith('Error: ')) {
					error = error.replace('Error: ', '');
				}
				errorMessage = 'Error while sending API request  ' + error;
				showErrorModal = true;
			}
		} catch (error) {
			let err = error instanceof Error ? error.message : String(error);
			if (typeof err === 'string' && err.startsWith('Error: ')) {
				err = err.replace('Error: ', '');
			}
			errorMessage = 'Error while toggling category:  ' + err;
			showErrorModal = true;
		} finally {
			isLoading.set(false);
			selectedCategories = [];
		}
	}

	let inputValue: string = '';
	let query: string = '';

	async function handleSearch(event?: KeyboardEvent) {
		if (event) {
			if (event.key === 'Enter' && !event.shiftKey) {
				event.preventDefault();
			} else {
				return;
			}
		}

		discoveryQuery.set(inputValue);
		query = inputValue;
		inputValue = '';

		isLoading.set(true);

		try {
			const res = await commands.sendDiscoveryRetrieverRequest(query, selectedCategories);

			if (res.status == 'ok') {
				if (res.data.insights.length === 0) {
					console.log('No results found');
					categories = [];
					return;
				}
				let discoveryData: DiscoveryDataPageList = {
					page_number: res.data.page_ranking,
					data: res.data.insights
				};

				discoverylist.set([discoveryData]);

				const insights = res.data.insights;

				if (insights) {
					categories = insights;
					discoveryApiResponseStore.set(insights);
				}
			} else {
				let error = res.error;
				if (typeof error === 'string' && error.startsWith('Error: ')) {
					error = error.replace('Error: ', '');
				}
				errorMessage = 'Error while sending the request  ' + error;
				isLoading.set(false);
				showErrorModal = true;
			}
		} catch (error) {
			let err = error instanceof Error ? error.message : String(error);
			if (typeof err === 'string' && err.startsWith('Error: ')) {
				err = err.replace('Error: ', '');
			}
			errorMessage = 'Error while performing search:  ' + err;
			isLoading.set(false);
			showErrorModal = true;
		} finally {
			isLoading.set(false);
		}
	}

	async function handlePrevious() {
		const discoveryData = get(discoverylist);
		const pageNumber = get(discoveryPageNumber) - 1;
		if (pageNumber <= 0) {
			return;
		}
		const previousPageData = discoveryData.find((item) => item.page_number === pageNumber);

		const insights = previousPageData?.data;

		if (previousPageData) {
			categories = previousPageData.data;
			discoveryApiResponseStore.set(previousPageData.data);
			discoveryPageNumber.set(pageNumber);
		} else {
			console.error('No data available for page number', pageNumber);
		}
	}

	async function handleNext() {
		const discoveryData = get(discoverylist);
		const pageNumber = get(discoveryPageNumber) + 1;
		const query = get(discoveryQuery);

		const nextPageData = discoveryData.find((item) => item.page_number === pageNumber);

		if (nextPageData) {
			categories = nextPageData.data;
			discoveryApiResponseStore.set(nextPageData.data);
			discoveryPageNumber.set(pageNumber);
		} else {
			try {
				isLoading.set(true);
				const res = await commands.sendDiscoveryRetrieverRequest(query, selectedCategories);

				if (res.status == 'ok') {
					if (res.data.insights.length === 0) {
						return;
					}
					let discoveryData: DiscoveryDataPageList = {
						page_number: pageNumber,
						data: res.data.insights
					};

					discoveryPageNumber.set(pageNumber);

					discoverylist.update((currentList) => {
						return [...currentList, discoveryData];
					});

					const insights = res.data.insights;

					if (insights) {
						discoveryApiResponseStore.set(insights);
						categories = insights;
					}
				} else {
					console.error('Error while sending the request', res.error);
				}
			} catch (error) {
				console.error('Error while fetching next page:', error);
			} finally {
				isLoading.set(false);
			}
		}
	}

	let showModal = false;
	let modalMessage = '';
	let isToggleOn = false;

	const toggleButton = document.getElementById('toggleButton') as HTMLInputElement;

	async function handleToggleOn(): Promise<void> {
		await tick();
		setTimeout(() => {
			isToggleOn = false;
		}, 400);

		modalMessage = 'This feature is available only in premium';
		showModal = true;
	}

	function handleToggle(event: Event): void {
		const target = event.target as HTMLInputElement;
		if (target.checked) {
			handleToggleOn();
		} else {
			console.log('Toggle is turned off');
		}
	}

	function extractFileNameWithExtension(filePath: string): string {
		const parts = filePath.split('/');
		return parts[parts.length - 1];
	}

	function getIconForDocument(documentName: string): string {
		const extension = documentName.split('.').pop()?.toLowerCase() || '';
		const iconMap: { [key: string]: string } = {
			pdf: 'vscode-icons:file-type-pdf2',
			doc: 'icon-park-outline:file-doc',
			docx: 'bi:filetype-docx',
			xls: 'ph:file-xls',
			xlsx: 'bi:filetype-xlsx',
			ppt: 'ph:file-ppt',
			pptx: 'bi:filetype-pptx',
			txt: 'ph:file-txt-light',
			jpg: 'ph:file-jpg-light',
			jpeg: 'ph:file-jpeg-light'
		};

		return iconMap[extension] || 'vscode-icons:default-file';
	}
	function formatDisplayText(text: string): string {
		return text
			.split(' ')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ');
	}
</script>

<main>
	<div class="search-wrapper">
		<div class="center-container">
			<div class="content-wrapper">
				<div class="discovery-container">
					<Icon {icon} class={iconClass} />
					<span class="discovery-text">Discovery</span>
				</div>
			</div>
		</div>

		<div class="card-container">
			<div class="search-container">
				<div class="input-wrapper">
					<textarea
						placeholder="Search..."
						class="search-input"
						id="searchInput"
						bind:value={inputValue}
						on:keydown={handleSearch}
					/>
					<button type="button" class="search-button" on:click={() => handleSearch()}
						><svg
							class="arrow-icon"
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
								d="M12 6v13m0-13 4 4m-4-4-4 4"
							/>
						</svg>
					</button>
				</div>
				<div class="toggle-container">
					<label class="switch">
						<input
							type="checkbox"
							id="toggleButton"
							bind:checked={isToggleOn}
							on:change={handleToggle}
						/>
						<span class="slider round"></span>
					</label>
					<span class="traverser-text">Data Fabric Traverser</span>
				</div>
			</div>

			<div class="dropdown-container">
				<div class="left-container">
					<div class="filter-dropdown">
						<button class="filter-button">
							<span class="filter-content">
								<Icon icon="mi:filter" height="20px" width="20px" />
								<span class="arrow-down"></span>
							</span>
						</button>
						<div class="dropdown-content">
							{#if categoriesDropdown}
								{#each categoriesDropdown as category}
									<label class="category-item">
										<input
											type="checkbox"
											checked={selectedCategories.includes(category)}
											on:change={() => toggleCategory(category)}
										/>
										{formatDisplayText(category)}
									</label>
								{/each}
							{/if}
						</div>
					</div>
				</div>

				<div class="right-container">
					<button type="button" class="nav-link previous" on:click={handlePrevious}>Previous</button
					>
					<button type="button" class="nav-link next" on:click={handleNext}>Next</button>
				</div>
			</div>

			<div class="discovery-query">
				{#if $discoveryQuery != ''}
					{$discoveryQuery}
				{/if}
			</div>

			<div class="cards-grid">
				{#if currentCategory && Array.isArray(currentCategory)}
					{#each currentCategory as category, index}
						<div class={index % 3 === 0 ? 'full-width-card' : 'small-card'}>
							<div class="card-content">
								<div class="svg-container">
									<Icon
										icon={getIconForDocument(extractFileNameWithExtension(category.document))}
										style="width: 32px; height: 32px;"
									/>
								</div>
								<p class="tagline font-semibold">
									{extractFileNameWithExtension(category.document)}
								</p>
								<p class="main-paragraph">
									{category.sentence}
								</p>
							</div>
							<div class="card-footer">
								<hr class="divider" />
								<div class="footer-content">
									<div class="sources">
										<Icon icon="icon-park:data" />
										<span class="label-text">Source</span>
										<span class="tooltip">{category.source}</span>
									</div>
									<div class="documents">
										<Icon icon="flat-color-icons:document" />
										<span class="label-text">Tags</span>
										<span class="tooltip">{category.tags}</span>
									</div>
								</div>
							</div>
						</div>
					{/each}
				{/if}
			</div>
		</div>
	</div>

	<Modal bind:show={showModal} message={modalMessage} />
	{#if $isLoading}
		<LoadingModal message="Please wait while we get your data...." />
	{/if}

	{#if showErrorModal}
		<ErrorModal {errorMessage} closeModal={closeErrorModal} />
	{/if}
</main>

<style>
	main {
		display: flex;
		justify-content: center;
		padding: 20px;
		box-sizing: border-box;
	}

	.search-wrapper {
		width: 100%;
		max-width: 1200px;
		text-align: center;
		box-sizing: border-box;
		margin: 0 auto;
		padding: 20px;
		box-sizing: border-box;
	}

	.toggle-container {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		margin-top: 15px;
	}

	.switch {
		position: relative;
		display: inline-block;
		width: 55px;
		height: 30px;
	}

	.switch input {
		opacity: 0;
		width: 0;
		height: 0;
	}

	.switch input:checked + .slider {
		background-color: #007bff;
	}

	.slider {
		position: absolute;
		cursor: pointer;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background-color: #ccc;
		transition: 0.4s;
		border-radius: 30px;
	}

	.slider:before {
		position: absolute;
		content: '';
		height: 26px;
		width: 26px;
		left: 2px;
		bottom: 2px;
		background-color: white;
		transition: 0.4s;
		border-radius: 50px;
	}

	input:checked + .slider {
		background-color: #007bff;
	}

	input:checked + .slider:before {
		transform: translateX(24px);
	}

	.traverser-text {
		margin-left: 10px;
		font-size: 16px;
		font-weight: bold;
		color: #007bff;
	}

	.left-container {
		margin-right: auto;
	}

	.center-container {
		flex-grow: 1;
		display: flex;
		justify-content: center;
		align-items: center;
	}

	.content-wrapper {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
	}

	.discovery-container {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 20px 0;
		margin-bottom: 20px;
	}

	.discovery-text {
		font-size: 28px;
		text-align: center;
		font-weight: bold;
	}

	.search-container {
		position: relative;
		display: flex;
		flex-direction: column;
		width: 100%;
		max-height: 150px;
		background-color: #fff;
		border-radius: 10px;
		box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
		padding: 20px;
		margin-bottom: 25px;
	}

	.input-wrapper {
		position: relative;
		display: flex;
		flex-grow: 1;
		min-height: 40px;
	}

	.search-input {
		flex-grow: 1;
		min-height: 40px;
		max-height: 120px;
		padding: 12px;
		border: 1px solid #ccc;
		border-radius: 8px;
		background: #f8f9fa;
		outline: none;
		padding: 12px;
		resize: none;
		overflow-y: auto;
		margin-right: 70px;
		font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
		font-size: 16px;
		line-height: 1.5;
		overflow-y: hidden;
	}

	.search-input::placeholder {
		color: #6c757d;
		font-size: 14px;
		font-style: italic;
		content: 'Search documents by title, category...';
	}
	.search-input:focus {
		outline: none;
		box-shadow: none;
		overflow-y: auto;
	}

	.search-button {
		position: absolute;
		background: #007bff;
		right: 1px;
		top: 50%;
		transform: translateY(-50%);
		padding: 12px;
		width: 48px;
		height: 48px;
		border-radius: 12px;
		box-shadow: 0 6px 12px rgba(0, 0, 0, 0.1);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.search-button:hover {
		background-color: #0056b3;
		box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
	}

	.card-container {
		width: 100%;
		max-width: 1200px;
		margin: 0 auto;
		padding: 20px;
		box-sizing: border-box;
	}

	.filter-dropdown {
		position: relative;
		display: inline-block;
		left: 0px;
		padding-top: 10px;
	}

	.filter-button {
		background-color: #007bff;
		color: white;
		padding: 10px 20px;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 16px;
	}

	.filter-content {
		display: flex;
		align-items: center;
	}

	.arrow-down {
		border: solid white;
		border-width: 0 2px 2px 0;
		display: inline-block;
		padding: 3px;
		margin-left: 10px;
		transform: rotate(45deg);
	}

	.dropdown-content {
		display: none;
		position: absolute;
		left: 0;
		background-color: #f9f9f9;
		min-width: 300px;
		max-height: 250px;
		box-shadow: 0px 8px 16px 0px rgba(0, 0, 0, 0.2);
		z-index: 1;
		max-height: 300px;
		overflow-y: auto;
	}

	.filter-dropdown:hover .dropdown-content {
		display: block;
	}

	.category-item {
		display: block;
		padding: 10px;
		cursor: pointer;
		white-space: nowrap;
		border-radius: 6px;
		font-size: 14px;
		transition: background-color 0.2s ease;
	}

	.category-item:hover {
		background-color: #f0f0f0;
	}

	.cards-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 30px;
		padding-top: 30px;
	}

	.full-width-card {
		grid-column: 1 / -1;
	}

	.full-width-card,
	.small-card {
		display: flex;
		flex-direction: column;
		background-color: #fff;
		border-radius: 12px;
		padding: 20px;
		box-shadow: 0 6px 12px rgba(0, 0, 0, 0.1);
		transition:
			box-shadow 0.3s ease-in-out,
			transform 0.2s ease-in-out;
		height: 100%;
	}

	.full-width-card:hover,
	.small-card:hover {
		box-shadow: 0 8px 16px rgba(0, 0, 0, 0.15);
		transform: translateY(-3px);
	}

	.tagline {
		margin: 0;
		padding: 10px 0 5px;
		text-align: left;
		font-weight: 700;
		font-size: 18px;
		color: #333;
		line-height: 1.4;
		overflow-wrap: break-word;
	}

	.main-paragraph {
		margin: 0;
		padding: 10px 0;
		text-align: left;
		font-size: 14px;
		line-height: 1.6;
		color: #666;
		flex-grow: 1;
	}

	.svg-container {
		display: flex;
		justify-content: center;
		align-items: center;
		height: 40px;
		background-color: white;
		margin-bottom: 10px;
	}

	.small-card {
		flex: 1 1 50%;
		border: 1px;
	}

	.divider {
		width: 100%;
		border: none;
		border-top: 1px solid #e0e0e0;
		margin-bottom: 10px;
	}

	.footer-content {
		display: flex;
		justify-content: space-between;
		width: 100%;
	}

	.sources,
	.documents {
		display: flex;
		align-items: center;
		cursor: pointer;
	}

	.label-text {
		margin-left: 6px;
		font-size: 10px;
		color: black;
	}

	.tooltip {
		display: none;
		position: absolute;
		background-color: #13343b;
		color: white;
		text-align: center;
		padding: 5px 10px;
		border-radius: 6px;
		z-index: 1;
		font-size: 12px;
	}

	.sources:hover .tooltip,
	.documents:hover .tooltip {
		display: block;
	}

	.card-footer {
		padding: 10px;
		width: 100%;
	}

	.card-content {
		flex-grow: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.right-container {
		display: flex;
		gap: 20px;
	}

	.nav-link {
		color: #1a0dab;
		text-decoration: none;
		font-size: 14px;
		font-weight: normal;
	}

	.nav-link:hover {
		text-decoration: underline;
	}

	.previous::before {
		content: '‹ ';
	}

	.next::after {
		content: ' ›';
	}

	.dropdown-container {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.discovery-query {
		padding: 15px;
		font-size: 24px;
		border: 2px solid transparent;
		border-radius: 5px;
		margin-bottom: 20px;
		line-height: 1.4;
		letter-spacing: 0.5px;
		transition: all 0.3s ease;
	}

	.discovery-query:hover {
		background-color: #f8f9fa;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
		border-color: transparent;
	}
	.category-item input[type='checkbox'] {
		appearance: none;
		width: 16px;
		height: 16px;
		border: 2px solid #007bff;
		border-radius: 4px;
		transition: background-color 0.2s ease;
		margin-right: 10px;
		cursor: pointer;
	}

	.category-item input[type='checkbox']:checked {
		background-color: #007bff;
		border-color: #007bff;
	}

	.category-item input[type='checkbox']:checked::after {
		content: '✓';
		color: white;
		font-size: 12px;
		display: block;
		text-align: center;
	}
</style>
