<script lang="ts">
	import { Modal, Button } from 'flowbite-svelte';
	import type { InsightAnalystRequest, InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import { commands, type InsightQuery } from '../../../../service/bindings';
	import { insightSessionId, isLoadingInsight, messagesList } from '../../../../stores/appState';
	import { get } from 'svelte/store';

	export let show = false;
	export let insight: InsightInfo | null = null;
	export let insightsId: string | null = null;
	let sessionId: string;
	let graphName = '';
	let graphDescription = '';
	let isPrivate = true;
	let inputQuery = '';

	let loadingStatus: boolean;
	$: {
		loadingStatus = $isLoadingInsight;
	}

	let messages: { text: string; isUser: boolean }[] = [];

	$: if (show) {
		initializeGraphBuilder(insightinfo);
	}

	async function initializeGraphBuilder(insight: InsightInfo) {
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
							} 
							// else if (key == 'query') {
							// 	query = insight.additionalOptions[key].value.value; }
							else {
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
				// let response = await commands.promptInsightAnalyst(request);
				// if (response.status == 'ok') {
				// 	responseModalMessage = response.data.response;
				// 	isResponseError = false;
				// } else {
				// 	responseModalMessage = response.error;
				// 	isResponseError = true;
				// 	console.error('Insight not running successfully');
				// }
				// isResponseModalOpen = true;
			} else {
				console.error('Insight not triggered successfully');
			}
		} catch (e) {
			console.error('Got error as ', e);
		} finally {
			isLoading.set(false);
		}
			let additional_options: { [x: string]: string } = {};
			let semantic_pipeline_id: string = '';
			let query: string = '';

			if (insight?.additionalOptions) {
				for (const key in insight.additionalOptions) {
					if (Object.prototype.hasOwnProperty.call(insight.additionalOptions, key)) {
						if (insight.additionalOptions[key].value.type === 'string') {
							if (key == 'semantic_pipeline_id') {
								semantic_pipeline_id = insight.additionalOptions[key].value.value;
							} 
							// else if (key == 'query') {
							// 	query = insight.additionalOptions[key].value.value; }
							else {
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
				// let response = await commands.promptInsightAnalyst(request);
				// if (response.status == 'ok') {
				// 	responseModalMessage = response.data.response;
				// 	isResponseError = false;
				// } else {
				// 	responseModalMessage = response.error;
				// 	isResponseError = true;
				// 	console.error('Insight not running successfully');
				// }
				// isResponseModalOpen = true;
			} else {
				console.error('Insight not triggered successfully');
			}
		} catch (e) {
			console.error('Got error as ', e);
		} finally {
			isLoading.set(false);
		}
	}

	async function generateGraph() {
		if (inputQuery.trim()) {
			messages = [...messages, { text: inputQuery, isUser: true }];
			messagesList.update((list) => [...list, { text: inputQuery, isUser: true }]);

			try {
				let request: InsightAnalystRequest = {
					id: insight?.id || '',
					discovery_session_id: '',
					semantic_pipeline_id: '',
					additional_options: { query: inputQuery, graphName, graphDescription, isPrivate: isPrivate ? 'true' : 'false' }
				};
				isLoadingInsight.set(true);

				let res = await commands.triggerInsightAnalyst(request);
				if (res.status === 'ok') {
					insightSessionId.set(res.data.session_id);
					sessionId = res.data.session_id;
					messages = [...messages, { text: 'Graph generated successfully!', isUser: false }];
				} else {
					console.error('Error while generating the graph:', res.error);
					messages = [...messages, { text: 'Error generating the graph. Please try again.', isUser: false }];
				}
			} catch (error) {
				console.error('Unexpected error while generating graph:', error);
			} finally {
				isLoadingInsight.set(false);
			}
		}
	}

	function closeModal() {
		show = false;
	}

	function handleKeyDown(event: { key: string; shiftKey: boolean; preventDefault: () => void }) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			generateGraph();
		}
	}
</script>

<Modal bind:open={show} size="xl" class="w-full" autoclose={false} on:close={closeModal}>
	<div class="mb-4 flex items-center justify-between">
		<h3 class="text-xl font-medium text-gray-900 dark:text-white">Graph Builder</h3>
	</div>
	<div class="mb-4 h-96 overflow-y-auto rounded-lg bg-gray-50 p-4 dark:bg-gray-700">
		<form class="space-y-4">
			<input
				class="w-full border border-gray-300 rounded-lg p-2"
				type="text"
				placeholder="Graph Name"
				bind:value={graphName}
			/>
			<textarea
				class="w-full border border-gray-300 rounded-lg p-2"
				placeholder="Graph Description"
				bind:value={graphDescription}
			></textarea>
			<div class="flex items-center">
				<label class="mr-2">Private</label>
				<input type="checkbox" bind:checked={isPrivate} />
			</div>
			<textarea
				class="w-full border border-gray-300 rounded-lg p-2"
				placeholder="Enter your query or context here..."
				bind:value={inputQuery}
				on:keydown={handleKeyDown}
			></textarea>
		</form>
		{#if loadingStatus}
			<div class="loader"></div>
		{/if}
	</div>
	<div class="flex justify-end mt-4">
		<Button on:click={generateGraph}>Generate Graph</Button>
	</div>
</Modal>

<style>
	.loader {
		border: 2px solid #f3f3f3;
		border-top: 2px solid #3498db;
		border-radius: 50%;
		width: 20px;
		height: 20px;
		animation: spin 1s linear infinite;
		position: relative;
	}

	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}
</style>
