<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import { onMount } from 'svelte';
	import { isVisible } from '../../../../stores/appState';
	import MetaTag from '../../../utils/MetaTag.svelte';
	import Modal from '../sources/add/Modal.svelte';
	import type { InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';

	const insightList: InsightInfo[] = [
		{
			id: 'insight1',
			name: 'Insight 1',
			description: 'Description for Insight 1',
			version: '1.0.0',
			conversational: true,
			author: 'John Doe',
			license: 'MIT',
			iconifyIcon: 'simple-icons:ollama',
			additionalOptions: {},
			premium: false
		},
		{
			id: 'insight2',
			name: 'Insight 2',
			description: 'Description for Insight 2',
			version: '1.1.0',
			conversational: false,
			author: 'Jane Smith',
			license: 'Apache-2.0',
			iconifyIcon: 'game-icons:laser-burst',
			additionalOptions: {},
			premium: true
		}
	];

	let selectedSource: string | null = null;

	let premiumSources = [
		'Azure',
		'Dropbox',
		'Email',
		'Github',
		'Jira',
		'News',
		'AWS S3',
		'Slack',
		'Google Cloud Storage'
	];

	let showModal = false;
	let modalMessage = '';

	function selectSource(sourceName: string) {
		if (premiumSources.includes(sourceName)) {
			modalMessage = 'This feature is only available in the premium version.';
			showModal = true;
		} else {
			$isVisible = true;
			selectedSource = sourceName;
		}
	}
	let selectedInsight: InsightInfo | null = null;

	function selectInsight(insight: InsightInfo) {
		if (insight.premium) {
			modalMessage = 'This feature is only available in the premium version.';
			showModal = true;
		} else {
			$isVisible = true;
			selectedInsight = insight;
		}
	}

	const path: string = '/crud/sources/add';
	const description: string = 'Add new source - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - Add New Source';
	const subtitle: string = 'Add New Source';
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem href="/crud/sources">Sources</BreadcrumbItem>
			<BreadcrumbItem>Add New Source</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			List of Insights
		</Heading>
		<div class="mt-6 grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-4">
			{#each insightList as insight}
				<button
					type="button"
					class="flex cursor-pointer flex-col items-center space-y-2"
					on:click={() => selectInsight(insight)}
					on:keydown={(event) => event.key === 'Enter' && selectInsight(insight)}
					aria-label={`Select ${insight.name}`}
				>
					<Icon icon={insight.iconifyIcon} width="32" height="32" />
					<span class="text-center text-gray-700 dark:text-gray-200">{insight.name}</span>
				</button>
			{/each}
		</div>

		<Modal bind:show={showModal} message={modalMessage} />
	</div>
</main>
