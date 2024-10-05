<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import MetaTag from '../../../utils/MetaTag.svelte';
	import type {
		CustomInsightOption,
		InsightAnalystRequest,
		InsightInfo,
		InsightQuery
	} from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import ChatModal from './ChatModal.svelte';
	import { commands } from '../../../../service/bindings';
	import { onMount } from 'svelte';
	import Modal from '../sources/add/Modal.svelte';
	import AdditionalOptionalModal from './AdditionalOptionalModal.svelte';
	import { messagesList, insightSessionId } from '../../../../stores/appState';
	import ErrorModal from '$lib/dashboard/ErrorModal.svelte';
	let showErrorModal = false;
	let errorMessage = '';
	function closeErrorModal() {
		showErrorModal = false;
	}
	import { writable } from 'svelte/store';
	import LoadingModal from './LoadingModal.svelte';
	import ResponseModal from './ResponseModal.svelte';


	let runningInsightId: string;
	let insightList: InsightInfo[];

	onMount(async () => {
		try {
			let res = await commands.listAvailableInsights();
			if (res.status == 'ok') {
				insightList = res.data;
			} else {
				errorMessage = 'Error fetching insights:' + res.error;
				showErrorModal = true;
			}
		} catch (error) {
			errorMessage = 'Unexpected error fetching insights:' + error;
			showErrorModal = true;
		}
	});

	$: runningInsightId = $insightSessionId;

	async function stopInsight() {
		try {
			let res = await commands.stopInsightAnalyst(runningInsightId);
			if (res.status == 'error') {
				errorMessage = 'Error while stopping the Insight:' + res.error;
				showErrorModal = true;
			}

			insightSessionId.set('');
			messagesList.set([]);
		} catch (error) {
			errorMessage = 'Unexpected error stopping the Insight:' + error;
			showErrorModal = true;
			messagesList.set([]);
		}
	}

	async function continueRunningInsight() {
		showChatModal = true;
	}

	let showAdditionalOptionModal = false;

	let showModal = false;
	let modalMessage = '';

	let showChatModal = false;
	let selectedInsightForChat: InsightInfo;

	function selectInsight(insight: InsightInfo) {
		if (insight.premium) {
			modalMessage = 'This feature is available only in premium';
			showModal = true;
		} else {
			if (runningInsightId && runningInsightId !== '') {
				modalMessage = 'You already have a running insight';
				showModal = true;
				return;
			}
			selectedInsightForChat = insight;
			showAdditionalOptionModal = true;
		}
	}

	const isLoading = writable(false);

	let responseModalMessage = '';
	let isResponseError = false;
	let isResponseModalOpen = false;
	async function triggerGraphBuilder(insight: InsightInfo) {
		try {
			let additional_options: { [x: string]: string } = {};
			let semantic_pipeline_id: string = '';
			let query: string = '';

			if (insight?.additionalOptions) {
				for (const key in insight.additionalOptions) {
					if (Object.prototype.hasOwnProperty.call(insight.additionalOptions, key)) {
						if (insight.additionalOptions[key].value.type === 'string') {
							if (key == 'semantic_pipeline_id') {
								semantic_pipeline_id = insight.additionalOptions[key].value.value;
							} else if (key == 'query') {
								query = insight.additionalOptions[key].value.value;
							} else {
								additional_options[key] = insight.additionalOptions[key].value
									.value as unknown as string;
							}
						}
					}
				}
			}
			let request: InsightAnalystRequest = {
				id: insight.id,
				discovery_session_id: '',
				semantic_pipeline_id: semantic_pipeline_id,
				additional_options: additional_options
			};
			isLoading.set(true);

			let res = await commands.triggerInsightAnalyst(request);
			if (res.status == 'ok') {
				console.log('Insight triggered successfully');
				let session_id = res.data.session_id;
				let request: InsightQuery = {
					session_id: session_id,
					query: query
				};
				let response = await commands.promptInsightAnalyst(request);
				if (response.status == 'ok') {
					responseModalMessage = response.data.response;
					isResponseError = false;
				} else {
					responseModalMessage = response.error;
					isResponseError = true;
					console.error('Insight not running successfully');
				}
				isResponseModalOpen = true;
			} else {
				console.error('Insight not triggered successfully');
			}
		} catch (e) {
			console.error('Got error as ', e);
		} finally {
			isLoading.set(false);
		}
	}

	async function handleSubmitOtions(event: CustomEvent<{ [key: string]: CustomInsightOption }>) {
		if (selectedInsightForChat.id == 'querent.insights.graph_builder.gbv1') {
			selectedInsightForChat.additionalOptions = event.detail;
			await triggerGraphBuilder(selectedInsightForChat);
		} else {
			selectedInsightForChat.additionalOptions = event.detail;
			showChatModal = true;
		}
	}

	function handleCloseAdditionalOptions() {
		showAdditionalOptionModal = false;
	}

	const path: string = '/crud/insights/';
	const description: string = 'Start Insights - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - Start Insights';
	const subtitle: string = 'Add Insights';
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="main-content p-4">
		{#if runningInsightId}
			<div class="button-container">
				<button on:click={continueRunningInsight} class="continue-button">Running Insight</button>
				<button on:click={stopInsight} class="stop-button">Stop Insight</button>
			</div>
		{/if}

		<div class="p-4">
			<Breadcrumb class="mb-5">
				<BreadcrumbItem home>Home</BreadcrumbItem>
				<BreadcrumbItem href="/crud/sources">Insights</BreadcrumbItem>
				<BreadcrumbItem>Start New Insight</BreadcrumbItem>
			</Breadcrumb>
			<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
				List of Insights
			</Heading>
			<div class="insight-grid">
				{#if Array.isArray(insightList)}
					{#each insightList as insight}
						<button
							type="button"
							class="insight-button"
							on:click={() => selectInsight(insight)}
							on:keydown={(event) => event.key === 'Enter' && selectInsight(insight)}
							aria-label={`Select ${insight.name}`}
						>
							<div class="insight-icon">
								<Icon icon={insight.iconifyIcon} style="width: 32px; height: 32px;" />
							</div>
							<div class="insight-content">
								<span class="insight-name">{insight.name}</span>
								{#if insight.description}
									<span class="insight-description">{insight.description}</span>
								{/if}
							</div>
						</button>
					{/each}
				{:else}
					{'Insight list is not an Array'}
				{/if}
			</div>

			<ChatModal
				bind:show={showChatModal}
				insight={selectedInsightForChat}
				insightsId={runningInsightId}
			/>
			<Modal bind:show={showModal} message={modalMessage} />

			{#if selectedInsightForChat}
				<AdditionalOptionalModal
					bind:show={showAdditionalOptionModal}
					insightInfo={selectedInsightForChat}
					on:submit={handleSubmitOtions}
					on:close={handleCloseAdditionalOptions}
				/>
			{/if}

			{#if $isLoading}
				<LoadingModal message="Please wait while we get your data...." />
			{/if}

			<ResponseModal
				message={responseModalMessage}
				isError={isResponseError}
				bind:isOpen={isResponseModalOpen}
			/>
		</div>
	</div>

	{#if showErrorModal}
		<ErrorModal {errorMessage} closeModal={closeErrorModal} />
	{/if}
</main>

<style>
	.insight-grid {
		margin-top: 1.5rem;
		display: flex;
		flex-wrap: wrap;
		gap: 2rem;
	}

	@media (min-width: 640px) {
		.insight-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	@media (min-width: 1024px) {
		.insight-grid {
			grid-template-columns: repeat(4, 1fr);
		}
	}

	.insight-button {
		display: flex;
		align-items: flex-start;
		cursor: pointer;
		padding: 1rem;
		border-radius: 0.5rem;
		transition: background-color 0.3s;
		width: calc(50% - 1rem);
	}

	.insight-button:hover {
		background-color: rgb(235, 225, 225);
	}
	.insight-icon {
		margin-right: 1rem;
	}
	.insight-content {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		text-align: left;
	}

	.insight-name {
		color: #374151;
		font-size: 1.125rem;
		margin-bottom: 0.25rem;
	}

	.insight-description {
		color: black;
		font-size: 0.875rem;
	}

	@media (min-width: 640px) {
		.insight-button {
			width: calc(33.333% - 1.333rem);
		}
	}

	@media (min-width: 1024px) {
		.insight-button {
			width: calc(25% - 1.5rem);
		}
	}

	@media (prefers-color-scheme: dark) {
		.insight-button:hover {
			background-color: rgb(241, 241, 241);
		}

		.insight-name {
			color: black;
		}

		.insight-description {
			color: black;
		}
	}

	.main-content {
		padding-top: 10px;
	}

	.button-container {
		position: absolute;
		top: 20px;
		right: 30px;
		display: flex;
		gap: 10px;
	}

	.stop-button,
	.continue-button {
		padding: 10px 20px;
		background-color: blue;
		color: white;
		border: none;
		border-radius: 5px;
		font-size: 16px;
		cursor: pointer;
	}
</style>
