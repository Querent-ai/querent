<script lang="ts">
	import { Modal, Button } from 'flowbite-svelte';
	import type { InsightAnalystRequest, InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import { commands, type InsightQuery } from '../../../../service/bindings';
	import { insightSessionId, isLoadingInsight, messagesList } from '../../../../stores/appState';
	import { get } from 'svelte/store';

	export let show = false;
	export let insight: InsightInfo;
	export let insightsId: string | null = null;
	let sessionId: string;
	let responseMessage = '';
	let errorMessage = '';
	let userQuery = '';
	let totalNodes = 0;
	let totalRelationships = 0;
	let graphDensity = 0;
	let totalCommunities = 0;
	let largestCommunitySize = 0;
	let topCentralNodes: [string, number][] = [];

	let loadingStatus: boolean;
	$: {
		loadingStatus = $isLoadingInsight;
	}
	let inputMessage = '';

	let icon: string;
	let description: string;

	$: if (show) {
		icon = insight?.iconifyIcon || '';
		description = insight?.description || '';
		initializeChat();
	}

	async function initializeChat() {
		try {
			if (insightsId && insightsId !== '') {
				sessionId = $insightSessionId;
			} else {
				responseMessage = '';
				errorMessage = '';
				let additional_options: { [x: string]: string } = {};
				let semantic_pipeline_id: string = '';
				if (insight?.additionalOptions) {
					for (const key in insight.additionalOptions) {
						if (Object.prototype.hasOwnProperty.call(insight.additionalOptions, key)) {
							if (insight.additionalOptions[key].value.type === 'string') {
								if (key == 'semantic_pipeline_id') {
									semantic_pipeline_id = insight.additionalOptions[key].value.value;
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
				isLoadingInsight.set(true);
				let res = await commands.triggerInsightAnalyst(request);
				if (res.status == 'ok') {
					insightSessionId.set(res.data.session_id);
					sessionId = res.data.session_id;
				} else {
					errorMessage = formatErrorMessage(res.error);
				}
			}
		} catch (error) {
			errorMessage = formatErrorMessage(String(error));
		} finally {
			isLoadingInsight.set(false);
		}
	}

	async function sendMessage() {
		try {
			isLoadingInsight.set(true);
			responseMessage = '';
			errorMessage = '';
			userQuery = inputMessage;
			totalNodes = 0;
			totalRelationships = 0;
			graphDensity = 0;
			totalCommunities = 0;
			largestCommunitySize = 0;
			topCentralNodes = [];
			let request: InsightQuery = {
				session_id: sessionId,
				query: inputMessage
			};
			let res = await commands.promptInsightAnalyst(request);
			if (res.status == 'ok') {
				const responseData = res.data.response;
				const cleanedData = responseData.replace(/\\n/g, '\n').replace(/\\"/g, '"');
				const totalNodesMatch = cleanedData.match(/Total Nodes: (\d+)/);
				const averageNodeDegree = cleanedData.match(/Average Node Degree: (\d+)/);
				const graphDensityMatch = cleanedData.match(/Graph Density: ([0-9.]+)/);
				const totalCommunitiesMatch = cleanedData.match(/Number of Communities: (\d+)/);
				const largestCommunitySizeMatch = cleanedData.match(/Largest Community Size: (\d+)/);

				totalNodes = totalNodesMatch ? parseInt(totalNodesMatch[1]) : 0;
				totalRelationships = averageNodeDegree ? parseInt(averageNodeDegree[1]) : 0;
				graphDensity = graphDensityMatch ? parseFloat(graphDensityMatch[1]) : 0;
				totalCommunities = totalCommunitiesMatch ? parseInt(totalCommunitiesMatch[1]) : 0;
				largestCommunitySize = largestCommunitySizeMatch
					? parseInt(largestCommunitySizeMatch[1])
					: 0;
				const topCentralNodesMatch = cleanedData.match(/Top 3 Central Nodes: \[(.+)\]/);
				if (topCentralNodesMatch) {
					const nodesString = topCentralNodesMatch[1].replace(/[\[\]\(\)\"]/g, '').trim();
					const nodesArray = nodesString.split(', ').reduce(
						(acc, val, idx, arr) => {
							if (idx % 2 === 0) {
								const nodeName = val;
								const connections = parseInt(arr[idx + 1]);
								if (!isNaN(connections)) {
									acc.push([nodeName.trim(), connections]);
								}
							}
							return acc;
						},
						[] as [string, number][]
					);
					topCentralNodes = nodesArray.filter(([nodeName, connections]) => !isNaN(connections));
				}
				responseMessage = cleanedData;
			} else {
				errorMessage = formatErrorMessage(res.error);
			}
		} catch (error) {
			console.error('Unexpected error while sending message:', error);
			errorMessage = formatErrorMessage(String(error));
		} finally {
			inputMessage = '';
			isLoadingInsight.set(false);
		}
	}

	function closeModal() {
		show = false;
	}

	function formatErrorMessage(error: string): string {
		if (error.includes('Received empty response')) {
			return 'No data was returned from the insight. Please check your input and try again.';
		}
		return 'An error occurred while processing your request: ' + error;
	}

	function handleKeyDown(event: { key: string; shiftKey: boolean; preventDefault: () => void }) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	}
</script>

<Modal
	bind:open={show}
	size="xl"
	class="h-auto max-h-[95vh] w-full"
	autoclose={false}
	on:close={closeModal}
>
	<div class="modal-header mb-4 flex items-center justify-between">
		<h3 class="text-xl font-medium text-gray-900 dark:text-white">
			{insight ? insight.name : 'Query'}
		</h3>
	</div>
	<div class="modal-body h-auto">
		<form on:submit|preventDefault={sendMessage} class="flex">
			<textarea
				placeholder="Enter your query or context..."
				class="search-input"
				id="searchInput"
				bind:value={inputMessage}
				on:keydown={handleKeyDown}
			></textarea>
			{#if loadingStatus}
				<div class="loader mr-2"></div>
			{/if}
			<Button type="submit">Send</Button>
		</form>
		{#if responseMessage}
			{#if userQuery}
				<div class="mb-4 rounded-lg bg-gray-100 p-4">
					<p class="text-sm text-gray-600"><strong>User Query:</strong> {userQuery}</p>
				</div>
			{/if}
			<div class="mt-8">
				<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
					<div class="card flex items-center rounded-lg bg-white p-4 shadow-md">
						<div
							class="mr-4 flex h-12 w-12 items-center justify-center rounded-full bg-blue-500 text-white"
						>
							<Icon icon="fluent:diagram-24-filled" class="h-6 w-6" />
						</div>
						<div>
							<h4 class="text-lg font-semibold text-gray-900">Total Nodes</h4>
							<p class="text-gray-700">{totalNodes}</p>
						</div>
					</div>
					<div class="card flex items-center rounded-lg bg-white p-4 shadow-md">
						<div
							class="mr-4 flex h-12 w-12 items-center justify-center rounded-full bg-green-500 text-white"
						>
							<Icon icon="fluent:link-20-filled" class="h-6 w-6" />
						</div>
						<div>
							<h4 class="text-lg font-semibold text-gray-900">Average Node Degree</h4>
							<p class="text-gray-700">{totalRelationships}</p>
						</div>
					</div>
					<div class="card flex items-center rounded-lg bg-white p-4 shadow-md">
						<div
							class="mr-4 flex h-12 w-12 items-center justify-center rounded-full bg-yellow-500 text-white"
						>
							<Icon icon="mdi:grid-large" class="h-6 w-6" />
						</div>
						<div>
							<h4 class="text-lg font-semibold text-gray-900">Graph Density</h4>
							<p class="text-gray-700">{graphDensity.toFixed(4)}</p>
						</div>
					</div>
				</div>

				<div class="mt-4 grid grid-cols-1 gap-6 md:grid-cols-3">
					<div class="card flex items-center rounded-lg bg-white p-4 shadow-md">
						<div
							class="mr-4 flex h-12 w-12 items-center justify-center rounded-full bg-purple-500 text-white"
						>
							<Icon icon="fluent:group-24-filled" class="h-6 w-6" />
						</div>
						<div>
							<h4 class="text-lg font-semibold text-gray-900">Communities Detected</h4>
							<p class="text-gray-700">{totalCommunities}</p>
						</div>
					</div>
					<div class="card flex items-center rounded-lg bg-white p-4 shadow-md">
						<div
							class="mr-4 flex h-12 w-12 items-center justify-center rounded-full bg-red-500 text-white"
						>
							<Icon icon="fluent:people-community-16-filled" class="h-6 w-6" />
						</div>
						<div>
							<h4 class="text-lg font-semibold text-gray-900">Largest Community Size</h4>
							<p class="text-gray-700">{largestCommunitySize}</p>
						</div>
					</div>
					<div class="card rounded-lg bg-white p-4 shadow-md">
						<div class="mb-0 flex items-center">
							<div
								class="mr-4 flex h-12 w-12 items-center justify-center rounded-full bg-orange-500 text-white"
							>
								<Icon icon="mdi:chart-donut" class="h-6 w-6" />
							</div>
							<h4 class="text-lg font-semibold text-gray-900">Top Central Nodes</h4>
						</div>
						<ul class="ml-[4rem] mt-0 space-y-1 text-gray-700">
							{#each topCentralNodes as [nodeName, connections]}
								<li>{nodeName}: {connections} connections</li>
							{/each}
						</ul>
					</div>
				</div>
			</div>
		{/if}
		{#if errorMessage}
			{#if userQuery}
				<div class="mb-4 rounded-lg bg-gray-100 p-4">
					<p class="text-sm text-gray-600"><strong>User Query:</strong> {userQuery}</p>
				</div>
			{/if}
			<div class="mb-4 rounded-lg bg-red-100 p-4">
				<p class="text-sm text-red-600"><strong>Error:</strong> {errorMessage}</p>
			</div>
		{/if}
	</div>
</Modal>

<style>
	:global(.modal-content > button[type='button']) {
		display: none !important;
	}

	.modal-body {
		padding: 1rem;
		background-color: #f9f9f9;
		border-radius: 10px;
	}

	.search-input {
		flex-grow: 1;
		min-height: 40px;
		max-height: 120px;
		padding: 10px;
		border: 1px solid #ddd;
		background: transparent;
		border-radius: 10px;
		box-shadow: 0px 2px 5px rgba(0, 0, 0, 0.1);
		transition:
			border-color 0.3s ease,
			box-shadow 0.3s ease;
		margin-right: 10px;
	}

	.search-input:focus {
		border-color: #3498db;
		box-shadow: 0px 2px 8px rgba(52, 152, 219, 0.4);
	}

	.modal-header {
		background-color: #f9f9f9;
		border-bottom: 1px solid #ddd;
		padding: 1rem;
		border-top-left-radius: 10px;
		border-top-right-radius: 10px;
	}

	.loader {
		border: 2px solid #f3f3f3;
		border-top: 2px solid #3498db;
		border-radius: 50%;
		width: 20px;
		height: 20px;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}

	.response {
		background-color: #ffffff;
		color: #333;
		border-radius: 8px;
		box-shadow: 0px 4px 6px rgba(0, 0, 0, 0.1);
	}
</style>
