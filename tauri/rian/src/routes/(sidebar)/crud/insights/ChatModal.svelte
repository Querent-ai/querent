<script lang="ts">
	import { Modal, Button } from 'flowbite-svelte';
	import type { InsightAnalystRequest, InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';
	import { commands, type InsightQuery } from '../../../../service/bindings-jsdoc';
	import { onMount } from 'svelte';
	import { discoverySessionId, pipelines, pipelineState } from '../../../../stores/appState';
	import { error } from 'console';
	import { get } from 'svelte/store';

	export let show = false;
	export let insight: InsightInfo | null = null;
	let sessionId: string;

	let messages: { text: string; isUser: boolean }[] = [];
	let inputMessage = '';

	let icon: string;
	let description: string;

	$: if (show) {
		icon = insight?.iconifyIcon || '';
		description = insight?.description || '';
	}

	onMount(async () => {
		try {
			let pipelineStateVar = get(pipelineState);
			if (!pipelineStateVar) {
				console.log('No running pipeline to run insights on');
				return;
			}
			let pipelineId = pipelineStateVar.id;
			console.log('Pipeline ID is ', pipelineId);

			let discoverySessionIdVar = get(discoverySessionId);
			if (!discoverySessionIdVar) {
				console.log('No discovery session found');
			}
			let id: string | undefined = insight?.id;
			let request: InsightAnalystRequest = {
				id: id!,
				discovery_session_id: discoverySessionIdVar,
				semantic_pipeline_id: pipelineId,
				additional_options: {}
			};
			let res = await commands.triggerInsightAnalyst(request);
			if (res.status == 'ok') {
				sessionId = res.data.session_id;
			} else {
				console.log('Got error while starting insights ', res.error);
			}
		} catch (error) {
			console.log('Got error while starting insights ', error);
		}
	});

	function sendMessage() {
		if (inputMessage.trim()) {
			messages = [...messages, { text: inputMessage, isUser: true }];
			inputMessage = '';
			setTimeout(async () => {
				let request: InsightQuery = {
					session_id: sessionId,
					query: inputMessage
				};
				let res = await commands.promptInsightAnalyst(request);
				if (res.status == 'ok') {
					messages = [...messages, { text: res.data.response, isUser: false }];
				} else {
					console.log('Error while calling insights ', res.error);
				}
			}, 1000);
		}
	}

	async function closeModal() {
		let res = await commands.stopInsightAnalyst('');
		show = false;
		messages = [];
	}
</script>

<Modal bind:open={show} size="xl" class="w-full" autoclose={false} on:close={closeModal}>
	<div class="mb-4 flex items-center justify-between">
		<h3 class="text-xl font-medium text-gray-900 dark:text-white">
			{insight ? insight.name : 'Chat'}
		</h3>
	</div>
	<div class="mb-4 h-96 overflow-y-auto rounded-lg bg-gray-50 p-4 dark:bg-gray-700">
		{#if messages.length === 0}
			<div class="flex h-full items-center justify-center">
				<div class="flex items-center">
					<Icon {icon} style="width: 48px; height: 48px;" />
					<p class="ml-4">{description}</p>
				</div>
			</div>
		{:else}
			{#each messages as message}
				<div class={`mb-2 ${message.isUser ? 'text-right' : 'text-left'}`}>
					<span
						class={`inline-block rounded-lg p-2 ${message.isUser ? 'bg-blue-500 text-white' : 'bg-gray-200 text-gray-700 dark:bg-gray-600 dark:text-white'}`}
					>
						{message.text}
					</span>
				</div>
			{/each}
		{/if}
	</div>
	<form on:submit|preventDefault={sendMessage} class="flex">
		<textarea
			placeholder="Search..."
			class="search-input"
			id="searchInput"
			bind:value={inputMessage}
		/>
		<Button type="submit">Send</Button>
	</form>
</Modal>

<style>
	:global(.modal-content > button[type='button']) {
		display: none !important;
	}
	.search-input {
		flex-grow: 1;
		min-height: 40px;
		max-height: 120px;
		padding: 10px;
		border: 1px solid black;
		background: transparent;
		outline: none;
		padding: 5px;
		resize: none;
		overflow-y: auto;
		margin-right: 50px;
		font-family: inherit;
		font-size: inherit;
		line-height: 1.5;
		overflow-y: hidden;
		border-radius: 20px;
	}

	.search-input:focus {
		outline: none;
		box-shadow: none;
		overflow-y: auto;
	}
	.flex.h-full.items-center.justify-center {
		height: 100%;
	}

	.flex.h-full.items-center.justify-center .flex.items-center {
		max-width: 80%;
	}

	.flex.h-full.items-center.justify-center p {
		margin-left: 1rem;
	}
</style>
