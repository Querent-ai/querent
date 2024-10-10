<script lang="ts">
	import { Modal, Button } from 'flowbite-svelte';
	import type { InsightAnalystRequest, InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import { commands, type InsightQuery } from '../../../../service/bindings';
	import { insightSessionId, isLoadingInsight, messagesList } from '../../../../stores/appState';
	import { get } from 'svelte/store';

	export let show = false;
	export let insight: InsightInfo ;
	export let insightsId: string | null = null;
	let sessionId: string;
	let responseMessage = '';

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
				let additional_options: { [x: string]: string } = {};
			let semantic_pipeline_id: string = '';
			// let query: string = '';

			if (insight?.additionalOptions) {
				for (const key in insight.additionalOptions) {
					if (Object.prototype.hasOwnProperty.call(insight.additionalOptions, key)) {
						if (insight.additionalOptions[key].value.type === 'string') {
							if (key == 'semantic_pipeline_id') {
								semantic_pipeline_id = insight.additionalOptions[key].value.value;
							}
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
			isLoadingInsight.set(true);
			console.log("This is the query-------", request);
			let res = await commands.triggerInsightAnalyst(request);
				if (res.status == 'ok') {
					insightSessionId.set(res.data.session_id);
					sessionId = res.data.session_id;
				} else {
					console.log('Error while starting insights:', res.error);
				}
			}
		} catch (error) {
			console.error('Unexpected error while initializing Graph Builder Insight:', error);
		} finally {
			isLoadingInsight.set(false);
		}
	}

	async function sendMessage() {
		console.log("This is the query-------", inputMessage);
			try {
					isLoadingInsight.set(true);
					let request: InsightQuery = {
					session_id: sessionId,
					query: inputMessage,
					};
					let res = await commands.promptInsightAnalyst(request);
					if (res.status == 'ok') {
						console.log("This is the query-------", res.data);
						responseMessage = res.data.response.replace(/\\n|\n/g, ' ').trim();
					} else {
						console.log('Error while processing the insight query:', res.error);
						responseMessage = formatErrorMessage(res.error);
					}				
			} catch (error) {
				console.error('Unexpected error while sending message:', error);
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

<Modal bind:open={show} size="xl" class="w-full" autoclose={false} on:close={closeModal}>
	<div class="modal-header mb-4 flex items-center justify-between">
		<h3 class="text-xl font-medium text-gray-900 dark:text-white">
			{insight ? insight.name : 'Query'}
		</h3>
	</div>
	<div class="modal-body">
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
			<div class="response mt-4 p-4 bg-white rounded-lg shadow">
				<h4 class="text-lg font-semibold mb-2">Response:</h4>
				<p class="text-gray-700 dark:text-white">{responseMessage}</p>
			</div>
		{/if}
	</div>
</Modal>

<style>
	:global(.modal-content > button[type='button']) {
		display: none !important;
	}

	.modal-body {
    max-height: 200px;
    overflow-y: auto;
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
		transition: border-color 0.3s ease, box-shadow 0.3s ease;
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