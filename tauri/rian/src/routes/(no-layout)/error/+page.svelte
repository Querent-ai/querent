<script>
	import { onMount } from 'svelte';
	let message = '';

	onMount(() => {
		const urlParams = new URLSearchParams(window.location.search);
		message = urlParams.get('error');
	});
</script>

<div class="error-container">
	<div class="error-title">Error</div>
	<div>{message}</div>
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
	}

	.error-title {
		color: red;
		margin-bottom: 0.5em;
	}
</style>
