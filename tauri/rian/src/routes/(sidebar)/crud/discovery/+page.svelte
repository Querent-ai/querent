<script lang="ts">
	import Icon from '@iconify/svelte';
	import PdfIcon from './PDFIcon.svelte';
	import {
		discoveryApiResponseStore,
		discoverylist,
		discoveryPageNumber,
		firstDiscovery,
		type DiscoveryDataPageList
	} from '../../../../stores/appState';
	import { get } from 'svelte/store';
	import { onMount, tick } from 'svelte';
	import { commands, type DiscoveryResponse } from '../../../../service/bindings';
	import Modal from '../sources/add/Modal.svelte';

	import FakeData from './fake_data.json';

	let categoriesDropdown: string[] | null;
	let currentCategory: {
		sentence: string;
		document: string;
		source: string;
		relationship_strength: string;
		tags: string;
		top_pairs: string[];
	}[];

	// const api_response: DiscoveryResponse = FakeData;
	$: categories =
		$discoveryApiResponseStore.length > 0 ? $discoveryApiResponseStore : FakeData.insights;
	// $: categories = api_response.insights;

	//only file name, top pairs

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

			const res = await commands.sendDiscoveryRetrieverRequest('', []);

			if (res.status == 'ok') {
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
				}
			}
		} else {
			console.log('We are coming here again');
			console.log('Data   ', discovery_data);
			console.log('Abcd   ', get(firstDiscovery));
			const currentList = get(discoverylist);

			const matchingPage = currentList.find(
				(item) => item.page_number === get(discoveryPageNumber)
			);

			const insights = matchingPage?.data;

			if (insights) {
				categories = insights;
			}
		}
	});

	let iconClass =
		'flex-shrink-0 w-6 h-6 text-gray-500 transition duration-75 group-hover:text-gray-900 dark:text-gray-400 dark:group-hover:text-white text-align:center';

	let icon = 'carbon:ibm-watson-discovery';

	let selectedCategories: any[] = [];

	async function toggleCategory(category: string) {
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
		}
	}

	let inputValue: string = '';
	let query: string = '';

	async function handleSearch(event?: KeyboardEvent) {
		if (event && event.key !== 'Enter') {
			return;
		}

		console.log('Selected categories  ', selectedCategories);
		console.log('User entered:', inputValue);
		query = inputValue;
		inputValue = '';

		const res = await commands.sendDiscoveryRetrieverRequest(query, selectedCategories);

		if (res.status == 'ok') {
			if (res.data.insights.length == 0) {
				console.log('Length is zero');
				return;
			}
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
		}
	}

	async function handlePrevious() {
		const discoveryData = get(discoverylist);
		const pageNumber = get(discoveryPageNumber);
		if (pageNumber < 1) {
			console.log('Already on first page');
			return;
		}
		const previousPageData = discoveryData.find((item) => item.page_number === pageNumber - 1);

		const insights = previousPageData?.data;

		if (insights) {
			discoveryApiResponseStore.set(insights);
		}
	}

	async function handleNext() {
		const res = await commands.sendDiscoveryRetrieverRequest(query, selectedCategories);

		if (res.status == 'ok') {
			if (res.data.insights.length == 0) {
				console.log('Length is zero');
				return;
			}
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
		}
	}

	let showModal = false;
	let modalMessage = '';
	let isToggleOn = false;

	const toggleButton = document.getElementById('toggleButton') as HTMLInputElement;

	async function handleToggleOn(): Promise<void> {
		console.log('Handle function');

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
					<span class="traverser-text">Pro</span>
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
										{category}
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

			<div class="cards-grid">
				{#if currentCategory && Array.isArray(currentCategory)}
					{#each currentCategory as category, index}
						<div class={index % 3 === 0 ? 'full-width-card' : 'small-card'}>
							<div class="card-content">
								<div class="svg-container">
									<PdfIcon />
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
		max-width: 800px;
		text-align: center;
		box-sizing: border-box;
		margin: 0 auto;
	}

	.toggle-container {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		margin-top: 10px;
	}

	.switch {
		position: relative;
		display: inline-block;
		width: 40px;
		height: 24px;
		border: 1px solid black;
		border-radius: 20px;
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
		background-color: #007bff;
		transition: 0.4s;
		border-radius: 24px;
	}

	.slider:before {
		position: absolute;
		content: '';
		height: 20px;
		width: 20px;
		left: 2px;
		bottom: 1px;
		background-color: #007bff;
		transition: 0.4s;
		border-radius: 50px;
	}

	input:checked + .slider {
		background-color: #2196f3;
	}

	input:checked + .slider:before {
		transform: translateX(16px);
	}

	.traverser-text {
		margin-left: 10px;
		color: #000;
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
		max-width: 600px;
	}

	.discovery-text {
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
		padding: 10px;
		overflow: visible;
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
		padding: 10px;
		border: none;
		background: transparent;
		outline: none;
		padding: 10px;
		resize: none;
		overflow-y: auto;
		margin-right: 50px;
		font-family: inherit;
		font-size: inherit;
		line-height: 1.5;
		overflow-y: hidden;
	}

	.search-input:focus {
		outline: none;
		box-shadow: none;
		overflow-y: auto;
	}

	.search-button {
		position: absolute;
		right: 4px;
		top: 50%;
		transform: translateY(-50%);
		background: none;
		border: none;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background-color: #007bff;
		padding: 8px;
		width: 40px;
		height: 40px;
	}

	.search-button:hover {
		background-color: rgb(46, 119, 235);
	}

	.card-container {
		width: 100%;
		max-width: 800px;
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
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.category-item:hover {
		background-color: #f1f1f1;
	}

	.cards-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 20px;
		padding-top: 20px;
	}

	.full-width-card {
		grid-column: 1 / -1;
	}

	.full-width-card,
	.small-card {
		display: flex;
		flex-direction: column;
		background-color: #fff;
		border-radius: 8px;
		padding: 15px;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
		transition: box-shadow 0.3s ease-in-out;
		height: 100%;
	}

	.full-width-card:hover,
	.small-card:hover {
		box-shadow: 0 6px 12px rgba(0, 0, 0, 0.15);
	}

	.tagline {
		margin: 0;
		padding: 10px 0 5px;
		text-align: left;
		font-weight: 600;
	}

	.main-paragraph {
		margin: 0;
		padding: 0 0 10px;
		text-align: left;
		font-size: 12px;
		flex-grow: 1;
	}

	.svg-container {
		display: flex;
		justify-content: center;
		align-items: center;
		height: 36px;
		background-color: white;
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
		position: relative;
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
		bottom: 100%;
		left: 50%;
		transform: translateX(-50%);
		font-size: 12px;
		min-height: 20px;
		min-width: 50px;
		overflow-x: inherit;
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
</style>
