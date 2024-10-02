<script>
	export let message = '';
	export let isError = false;
	export let isOpen = false;

	let modalContainer;

	function closeModal() {
		isOpen = false;
	}

	function handleKeyDown(event) {
		if (event.key === 'Escape') {
			closeModal();
		}
	}

	$: if (isOpen && modalContainer) {
		modalContainer.focus();
	}
</script>

{#if isOpen}
	<div class="modal-backdrop" on:click={closeModal} role="presentation">
		<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
		<div
			class="modal"
			bind:this={modalContainer}
			on:click|stopPropagation
			role="dialog"
			aria-modal="true"
			aria-labelledby="modalHeading"
			aria-describedby="modalDescription"
			on:keydown={handleKeyDown}
		>
			<h2 id="modalHeading">{isError ? 'Error' : 'Success'}</h2>
			<p id="modalDescription">{message.replace(/['"]+/g, '')}</p>
			<div class="button-container">
				<button on:click={closeModal} class="btn close-btn">Close</button>
			</div>
		</div>
	</div>
{/if}

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
	}

	.modal {
		background-color: white;
		padding: 20px;
		border-radius: 5px;
		max-width: 80%;
		outline: none;
		box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
	}

	h2 {
		margin-top: 0;
		font-size: 1.5rem;
	}
	p {
		font-size: 1rem;
	}

	.button-container {
		display: flex;
		justify-content: flex-end;
		margin-top: 20px;
	}

	.btn {
		padding: 10px 20px;
		border: none;
		border-radius: 5px;
		font-size: 1rem;
		cursor: pointer;
	}

	.close-btn {
		background-color: #f44336;
		color: white;
		margin-right: 10px;
	}

</style>
