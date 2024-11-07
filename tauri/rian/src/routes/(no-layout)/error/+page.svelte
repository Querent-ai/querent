<script>
	import { onMount } from 'svelte';
	let message = '';

	onMount(() => {
		const urlParams = new URLSearchParams(window.location.search);
		message = urlParams.get('error');

		if (message && message.startsWith('Error: ')) {
			message = message.replace('Error: ', '');
		}
	});
</script>

<div class="error-container">
	<div class="error-icon">⚠️</div>
	<h2 class="error-title">Something went wrong</h2>
	<p class="error-message">{message}</p>
	<a
		href="https://github.com/Querent-ai/support/issues/new?assignees=&labels=&projects=&template=support_help.md&title="
		target="_blank"
		class="report-link">Report this issue</a
	>
</div>

<style>
	.error-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100vh;
		text-align: center;
		font-size: 1.5em;
		color: #333;
		background-color: #fef3f3;
		padding: 2em;
		border: 1px solid #ffdddd;
		border-radius: 8px;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
	}

	.error-icon {
		font-size: 3em;
		margin-bottom: 0.5em;
		color: #ff6b6b;
	}

	.error-title {
		font-size: 1em;
		margin: 0.2em 0;
		color: #cc0000;
	}

	.error-message {
		font-size: 1.4em;
		color: #555;
		margin: 1em 0;
	}

	.report-link {
		font-size: 1em;
		color: #0073e6;
		text-decoration: underline;
		margin-top: 1em;
		cursor: pointer;
	}

	.report-link:hover {
		color: #005bb5;
	}
</style>
