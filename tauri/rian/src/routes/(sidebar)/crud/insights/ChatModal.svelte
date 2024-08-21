<script lang="ts">
	import { Modal, Button } from 'flowbite-svelte';
	import type { InsightInfo } from '../../../../service/bindings';
	import Icon from '@iconify/svelte';

	export let show = false;
	export let insight: InsightInfo | null = null;

	let messages: { text: string; isUser: boolean }[] = [];
	let inputMessage = '';

	let icon: string;
	let description: string;

	$: if (show) {
		icon = insight?.iconifyIcon || '';
		description = insight?.description || '';
	}

	function sendMessage() {
		if (inputMessage.trim()) {
			messages = [...messages, { text: inputMessage, isUser: true }];
			inputMessage = '';
			setTimeout(() => {
				messages = [...messages, { text: 'Hello from querent', isUser: false }];
			}, 1000);
		}
	}

	function closeModal() {
		show = false;
		messages = [];
	}

	console.log('description   ', insight?.description);
</script>

<Modal bind:open={show} size="xl" class="w-full" autoclose={false} on:close={closeModal}>
	<div class="mb-4 flex items-center justify-between">
		<h3 class="text-xl font-medium text-gray-900 dark:text-white">
			{insight ? insight.name : 'Chat'}
		</h3>
	</div>
	<div class="mb-4 h-96 overflow-y-auto rounded-lg bg-gray-50 p-4 dark:bg-gray-700">
		{#if messages.length === 0}
			<div class="flex h-full flex-col items-center justify-center">
				<Icon {icon} style="width: 48px; height: 48px;" />
				<p class="text-center">{description}</p>
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
</style>
