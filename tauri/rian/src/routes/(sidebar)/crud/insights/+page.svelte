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
	import GBModal from './GBModal.svelte';
	import { commands } from '../../../../service/bindings';
	import { onMount } from 'svelte';
	import Modal from '../sources/add/Modal.svelte';
	import AdditionalOptionalModal from './AdditionalOptionalModal.svelte';
	import { messagesList, insightSessionId } from '../../../../stores/appState';
	import { writable } from 'svelte/store';
	import LoadingModal from './LoadingModal.svelte';

	let runningInsightId: string;
	let insightList: InsightInfo[] = [];
	let selectedInsightForChat: InsightInfo;
	let showAdditionalOptionModal = false;
	let showModal = false;
	let modalMessage = '';
	let showChatModal = false;
	let showGBModal = false;
	const isLoading = writable(false);

	onMount(async () => {
		try {
			const res = await commands.listAvailableInsights();
			if (res.status === 'ok') {
				insightList = res.data.sort((a, b) => (a.premium ? 1 : 0) - (b.premium ? 1 : 0));
			} else {
				console.error('Error fetching insights:', res.error);
			}
		} catch (error) {
			console.error('Unexpected error fetching insights:', error);
		}
	});

	$: runningInsightId = $insightSessionId;

	async function stopInsight() {
		try {
			const res = await commands.stopInsightAnalyst(runningInsightId);
			if (res.status === 'error') {
				console.error('Error while stopping the Insight:', res.error);
			}
			insightSessionId.set('');
			messagesList.set([]);
		} catch (error) {
			console.error('Unexpected error stopping the Insight:', error);
			messagesList.set([]);
		}
	}

	async function continueRunningInsight() {
		if (selectedInsightForChat.id === 'querent.insights.graph_builder.gbv1') {
			showGBModal = true;
		} else {
			showChatModal = true;
		}
	}

	function selectInsight(insight: InsightInfo) {
		if (insight.premium) {
			modalMessage = 'This feature is available only in the Pro version.';
			showModal = true;
		} else {
			if (runningInsightId) {
				modalMessage = 'You already have a running insight.';
				showModal = true;
				return;
			}
			selectedInsightForChat = insight;
			showAdditionalOptionModal = true;
		}
	}

	async function handleSubmitOptions(event: CustomEvent<{ [key: string]: CustomInsightOption }>) {
		selectedInsightForChat.additionalOptions = event.detail;
		if (selectedInsightForChat.id === 'querent.insights.graph_builder.gbv1') {
			showGBModal = true;
		} else {
			showChatModal = true;
		}
	}

	function handleCloseAdditionalOptions() {
		showAdditionalOptionModal = false;
	}

	const path = '/crud/insights/';
	const description = 'Start Insights - Querent Admin Dashboard';
	const title = 'Querent Admin Dashboard - Start Insights';
	const subtitle = 'Add Insights';
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
				<BreadcrumbItem href="/crud/insights">Insights</BreadcrumbItem>
				<BreadcrumbItem>Start New Insight</BreadcrumbItem>
			</Breadcrumb>
			<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
				Insights
			</Heading>
			<div class="insight-grid">
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
							<span class="insight-name">
								{insight.name}
								{#if insight.premium}
									<span class="insight-badge pro">Pro</span>
								{/if}
							</span>
							{#if insight.description}
								<span class="insight-description">{insight.description}</span>
							{/if}
						</div>
					</button>
				{/each}
			</div>

			<ChatModal
				bind:show={showChatModal}
				insight={selectedInsightForChat}
				insightsId={runningInsightId}
			/>
			<GBModal
				bind:show={showGBModal}
				insight={selectedInsightForChat}
				insightsId={runningInsightId}
			/>
			<Modal bind:show={showModal} message={modalMessage} />

			{#if selectedInsightForChat}
				<AdditionalOptionalModal
					bind:show={showAdditionalOptionModal}
					insightInfo={selectedInsightForChat}
					on:submit={handleSubmitOptions}
					on:close={handleCloseAdditionalOptions}
				/>
			{/if}

			{#if $isLoading}
				<LoadingModal message="Please wait while we get your data...." />
			{/if}
		</div>
	</div>
</main>

<style>
	.insight-grid {
		margin-top: 1.5rem;
		display: grid;
		gap: 2rem;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
	}

	.insight-button {
		display: flex;
		align-items: flex-start;
		cursor: pointer;
		padding: 1rem;
		border-radius: 0.5rem;
		transition:
			background-color 0.3s,
			box-shadow 0.3s;
		background: white;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		width: 100%;
	}

	.insight-button:hover {
		background-color: #f9f9f9;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
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
		font-weight: 600;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		line-height: 1.2;
	}

	.insight-badge.pro {
		padding: 0.25rem 0.5rem;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
		background-color: #f97316;
		color: white;
		align-self: flex-start;
	}

	.insight-description {
		color: #6b7280;
		font-size: 0.875rem;
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

	.modal-close-button {
		background: #007bff;
		color: white;
		padding: 0.5rem 1rem;
		border-radius: 5px;
		margin-top: 1rem;
		cursor: pointer;
	}

	.stop-button,
	.continue-button {
		padding: 10px 20px;
		background-color: #007bff;
		color: white;
		border: none;
		border-radius: 5px;
		font-size: 16px;
		cursor: pointer;
	}

	.stop-button:hover,
	.continue-button:hover {
		background-color: #0056b3;
	}
</style>
