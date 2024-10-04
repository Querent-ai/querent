<script>
	import { onMount } from 'svelte';

	export let errorMessage = '';
	export let closeModal;

	let modalElement;

	function handleKeydown(event) {
		if (event.key === 'Escape') {
			closeModal();
		}
	}

	onMount(() => {
		if (modalElement) {
			modalElement.focus();
		}
	});
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="modal-backdrop" aria-modal="true" role="dialog" aria-labelledby="error-title">
	<div class="modal-content" role="document" bind:this={modalElement} tabindex="-1">
		<h2 id="error-title">Error</h2>
		<p id="error-message">{errorMessage}</p>
		<div class="button-group">
			<button on:click={closeModal} class="close-btn">Close</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		justify-content: center;
		align-items: center;
		z-index: 1000;
	}

	.modal-content {
		background-color: white;
		padding: 20px;
		border-radius: 5px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
		max-width: 400px;
		width: 100%;
	}

	h2 {
		margin-top: 0;
		color: #e74c3c;
	}

	.button-group {
		display: flex;
		justify-content: flex-end;
		margin-top: 20px;
	}

	button {
		margin-left: 10px;
		padding: 8px 16px;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 14px;
		transition: background-color 0.3s;
	}

	.close-btn {
		background-color: #007bff;
		color: black;
	}

	.close-btn:hover,
	.close-btn:focus {
		background-color: #006ce0;
	}

	button:focus {
		outline: 2px solid #3498db;
		outline-offset: 2px;
	}
</style>
