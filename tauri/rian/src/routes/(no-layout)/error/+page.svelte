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
</style>
