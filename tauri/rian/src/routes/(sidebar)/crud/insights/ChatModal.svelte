<script lang="ts">
	import { Modal, Button } from 'flowbite-svelte';
	import type { InsightAnalystRequest, InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import { commands, type InsightQuery } from '../../../../service/bindings';
	import { insightSessionId, isLoadingInsight, messagesList } from '../../../../stores/appState';
	import { get } from 'svelte/store';
	import ErrorModal from '$lib/dashboard/ErrorModal.svelte';
	let showErrorModal = false;
	let errorMessage = '';
	function closeErrorModal() {
		showErrorModal = false;
	}

	export let show = false;
	export let insight: InsightInfo | null = null;
	export let insightsId: string | null = null;
	let sessionId: string;

	let loadingStatus: boolean;
	$: {
		loadingStatus = $isLoadingInsight;
	}

	let messages: { text: string; isUser: boolean }[] = [];
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
				messages = [];
				let previousMessages = get(messagesList);
				previousMessages.forEach((message) => {
					messages = [...messages, { text: message.text, isUser: message.isUser }];
				});
				sessionId = $insightSessionId;
			} else {
				messages = [];
				let id: string | undefined = insight?.id;
				if (!id) {
					console.log('No id found');
					return;
				}
				let additional_options: { [x: string]: string } = {};

				if (insight?.additionalOptions) {
					for (const key in insight.additionalOptions) {
						if (Object.prototype.hasOwnProperty.call(insight.additionalOptions, key)) {
							if (insight.additionalOptions[key].value.type === 'string') {
								additional_options[key] = insight.additionalOptions[key].value
									.value as unknown as string;
							}
						}
					}
				}
				let request: InsightAnalystRequest = {
					id: id!,
					discovery_session_id: '',
					semantic_pipeline_id: '',
					additional_options: additional_options
				};
				isLoadingInsight.set(true);

				let res = await commands.triggerInsightAnalyst(request);
				if (res.status == 'ok') {
					insightSessionId.set(res.data.session_id);
					sessionId = res.data.session_id;
				} else {
					let err = res.error as string;
					if (typeof err === 'string' && err.startsWith('Error: ')) {
						err = err.replace('Error: ', '');
					}
					errorMessage = 'Error while starting insights:' + err;
					showErrorModal = true;
				}
			}
		} catch (error) {
			let err = error instanceof Error ? error.message : String(error);
			if (typeof err === 'string' && err.startsWith('Error: ')) {
				err = err.replace('Error: ', '');
			}
			errorMessage = 'Unexpected error while initializing chat:' + err;
			showErrorModal = true;
		} finally {
			isLoadingInsight.set(false);
		}
	}

	async function sendMessage() {
		if (inputMessage.trim()) {
			messages = [...messages, { text: inputMessage, isUser: true }];
			messagesList.update((list) => [...list, { text: inputMessage, isUser: true }]);

			let query = inputMessage;
			try {
				setTimeout(async () => {
					let request: InsightQuery = {
						session_id: sessionId,
						query: query
					};
					isLoadingInsight.set(true);
					let res = await commands.promptInsightAnalyst(request);
					if (res.status == 'ok') {
						let text = res.data.response
							.replace(/\\n|\n/g, ' ')
							.replace(/\s+/g, ' ')
							.trim();
						if (text.startsWith('"') && text.endsWith('"')) {
							text = text.slice(1, -1);
						}
						messages = [...messages, { text: text, isUser: false }];

						messagesList.update((list) => [...list, { text: text, isUser: false }]);
					} else {
						let err = res.error as string;
						if (typeof err === 'string' && err.startsWith('Error: ')) {
							err = err.replace('Error: ', '');
						}
						errorMessage = 'Error while processing the insight query: ' + err;
						showErrorModal = true;
					}
				}, 100);
				inputMessage = '';
			} catch (error) {
				let err = error instanceof Error ? error.message : String(error);
				if (typeof err === 'string' && err.startsWith('Error: ')) {
					err = err.replace('Error: ', '');
				}
				errorMessage = 'Unexpected error while sending message:' + err;
				isLoadingInsight.set(false);
				showErrorModal = true;
			} finally {
				isLoadingInsight.set(false);
			}
		}
	}

	function closeModal() {
		show = false;
	}

	function formatMessageText(text: string) {
		return text.replace(/\n/g, '<br>');
	}

	function handleKeyDown(event: { key: string; shiftKey: boolean; preventDefault: () => void }) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	}
</script>

<Modal bind:open={show} size="xl" class="w-full" autoclose={false} on:close={closeModal}>
	<div class="mb-4 flex items-center justify-between">
		<h3 class="text-xl font-medium text-gray-900 dark:text-white">
			{insight ? `Querent Chat - ${insight.name}` : 'Chat'}
		</h3>
	</div>
	<div class="mb-4 h-80 overflow-y-auto rounded-lg bg-gray-50 p-4 dark:bg-gray-700">
		{#if messages.length === 0}
			<div class="flex h-full items-center justify-center">
				<div class="flex items-center text-center">
					<Icon {icon} style="width: 48px; height: 48px;" />
					<p class="ml-4 text-gray-500">{description}</p>
				</div>
			</div>
		{:else}
			{#each messages as message}
				<div class={`mb-2 ${message.isUser ? 'text-right' : 'text-left'}`}>
					<span
						class={`inline-block rounded-lg p-2 ${message.isUser ? 'bg-blue-500 text-white' : 'bg-gray-200 text-gray-700 dark:bg-gray-600 dark:text-white'}`}
					>
						{@html formatMessageText(message.text)}
					</span>
				</div>
			{/each}
		{/if}
	</div>
	<form
		on:submit|preventDefault={sendMessage}
		class="flex items-center rounded-b-lg bg-gray-100 p-2"
	>
		<textarea
			placeholder="Type your message here..."
			class="search-input"
			id="searchInput"
			bind:value={inputMessage}
			on:keydown={handleKeyDown}
		/>
		{#if loadingStatus}
			<div class="loader mr-2"></div>
		{/if}
		<Button type="submit" class="ml-2">Send</Button>
	</form>
</Modal>

{#if showErrorModal}
	<ErrorModal {errorMessage} closeModal={closeErrorModal} />
{/if}

<style>
	:global(.modal-content > button[type='button']) {
		display: none !important;
	}

	.search-input {
		flex-grow: 1;
		min-height: 40px;
		max-height: 80px;
		padding: 10px;
		border: 1px solid #ddd;
		background: #fff;
		outline: none;
		resize: none;
		border-radius: 4px;
		font-size: 0.95rem;
	}

	.search-input:focus {
		border-color: #3498db;
		box-shadow: 0px 2px 8px rgba(52, 152, 219, 0.2);
	}

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

	.flex.h-full.items-center.justify-center {
		height: 100%;
	}

	.flex.h-full.items-center.justify-center .flex.items-center {
		max-width: 80%;
		position: relative;
	}

	.flex.h-full.items-center.justify-center p {
		margin-left: 1rem;
		color: #666;
		font-size: 0.9rem;
	}
</style>
