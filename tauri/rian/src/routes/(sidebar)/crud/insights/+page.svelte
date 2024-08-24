<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import MetaTag from '../../../utils/MetaTag.svelte';
	import type {
		CustomInsightOption,
		InsightCustomOptionValue,
		InsightInfo
	} from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import ChatModal from './ChatModal.svelte';
	import { commands, type InsightAnalystRequest } from '../../../../service/bindings-jsdoc';
	import { onMount } from 'svelte';
	import Modal from '../sources/add/Modal.svelte';
	import AdditionalOptionalModal from './AdditionalOptionalModal.svelte';

	let runningInsight: string;
	let runningInsightData;

	let insightList: InsightInfo[];
	onMount(async () => {
		let res = await commands.listAvailableInsights();
		if (res.status == 'ok') {
			insightList = res.data;
			console.log('available insights are ', res.data);
			console.log('Addtional input   ', insightList[0].additionalOptions);
		} else {
			console.log('Error as ', res.error);
		}

		let resInsights = await commands.getRunningInsightAnalysts();
		let runningInsightList = resInsights.map(([id, request]) => id);
		let data = resInsights.map(([id, request]) => request);
		if (runningInsightList.length > 0) {
			runningInsight = runningInsightList[0];
			runningInsightData = data;
		}
	});

	async function stopInsight() {
		let res = await commands.stopInsightAnalyst(runningInsight);
		if (res.status == 'error') {
			console.log('Error while stopping the Insight');
		}
	}

	async function continueRunningInsight() {
		showChatModal = true;
	}

	let showAdditionalOptionModal = false;
	function launchModal(insight: InsightInfo) {
		selectedInsightForChat = insight;
		showAdditionalOptionModal = true;
	}

	let showModal = false;
	let modalMessage = '';

	let showChatModal = false;
	let selectedInsightForChat: InsightInfo;

	function selectInsight(insight: InsightInfo) {
		if (insight.premium) {
			console.log('Premium insight');
			modalMessage = 'This feature is available only in premium';
			showModal = true;
		} else {
			selectedInsightForChat = insight;
			showAdditionalOptionModal = true;
		}
	}

	function handleSubmitOtions(event: CustomEvent<{ [key: string]: CustomInsightOption }>) {
		console.log('Additional information submitted:', event.detail);
		selectedInsightForChat.additionalOptions = event.detail;
		console.log('Insight from function ', selectedInsightForChat);
		showChatModal = true;
	}
	function handleCloseAdditionalOptions() {
		showModal = false;
	}

	const path: string = '/crud/insights/';
	const description: string = 'Start Insights - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - Start Insights';
	const subtitle: string = 'Add Insights';
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	{#if runningInsight}
		<button on:click={stopInsight} class="stop-button"> Stop Insight </button>
		<button on:click={continueRunningInsight} class="continue-button"> Continue Insights </button>
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
			insightsId={runningInsight}
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
	</div>
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
		background-color: black;
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

	.stop-button {
		position: fixed;
		top: 110px;
		right: 30px;
		padding: 10px 20px;
		background-color: blue;
		color: white;
		border: none;
		border-radius: 5px;
		font-size: 16px;
		cursor: pointer;
		z-index: 1000;
	}
	.continue-button {
		position: fixed;
		top: 110px;
		right: 160px;
		padding: 10px 20px;
		background-color: blue;
		color: white;
		border: none;
		border-radius: 5px;
		font-size: 16px;
		cursor: pointer;
		z-index: 1000;
	}
</style>
