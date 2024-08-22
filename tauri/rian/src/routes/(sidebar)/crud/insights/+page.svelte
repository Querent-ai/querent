<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import MetaTag from '../../../utils/MetaTag.svelte';
	import type { InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import ChatModal from './ChatModal.svelte';

	const insightList: InsightInfo[] = [
		{
			id: 'querent.insights.x_ai.claude',
			name: 'Querent xAI with Claude3pus20240229',
			description:
				"xAI utilizes generative models to perform a directed traversal in R!AN's attention data fabric.",
			version: '1.0.0',
			author: 'Querent AI',
			license: 'Apache-2.0',
			iconifyIcon: 'game-icons:laser-burst',
			additionalOptions: {},
			conversational: true,
			premium: false
		},
		{
			id: 'querent.insights.x_ai.ollama',
			name: 'Querent xAI with Ollama',
			description:
				"xAI utilizes generative models to perform a directed traversal in R!AN's attention data fabric.",
			version: '1.0.0',
			author: 'Querent AI',
			license: 'Apache-2.0',
			iconifyIcon: 'simple-icons:ollama',
			additionalOptions: {},
			conversational: true,
			premium: false
		},
		{
			id: 'querent.insights.x_ai.openai',
			name: 'Querent xAI with GPT35 Turbo',
			description:
				"xAI utilizes generative models to perform a directed traversal in R!AN's attention data fabric.",
			version: '1.0.0',
			author: 'Querent AI',
			license: 'Apache-2.0',
			iconifyIcon: 'ri:openai-fill',
			additionalOptions: {},
			conversational: true,
			premium: false
		}
	];

	let showModal = false;
	let modalMessage = '';

	let showChatModal = false;
	let selectedInsightForChat: InsightInfo;

	function selectInsight(insight: InsightInfo) {
		if (insight.premium) {
			modalMessage = 'This feature is available only in premium';
			showModal = true;
		} else {
			showChatModal = true;
			selectedInsightForChat = insight;

			console.log('desc is this fucking thing  ', insight.iconifyIcon);
		}
	}

	const path: string = '/crud/insights/';
	const description: string = 'Start Insights - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - Start Insights';
	const subtitle: string = 'Add Insights';
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
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
		</div>

		<ChatModal bind:show={showChatModal} insight={selectedInsightForChat} />
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
</style>
