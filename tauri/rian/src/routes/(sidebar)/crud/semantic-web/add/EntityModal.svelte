<script>
	export let show = false;
	export let entitiesPairs = [];

	function close() {
		show = false;
	}

	function handleKeydown(event) {
		if (event.key === 'Escape') {
			close();
		}
	}

	function removeEntityPair(index) {
		entitiesPairs.splice(index, 1);
		entitiesPairs = entitiesPairs.slice();
	}
</script>

{#if show}
	<div role="dialog" aria-modal="true" class="modal-overlay">
		<!-- <section
			role="document"
			class="modal-content"
			on:click|stopPropagation
			on:keydown={handleContentKeydown}
			tabindex="-1"
		>
			<h2 class="modal-title">Entity Pairs</h2>
			<div class="bubbles-container">
				{#each entitiesPairs as pair, index}
					<div class="entity-bubble">
						{pair.entity} - {pair.entityType}
						<button
							class="delete-btn"
							on:click={() => removeEntityPair(index)}
							aria-label="Remove entity pair">&times;</button
						>
					</div>
				{/each}
			</div>
			<button class="close-btn" on:click={close}>Close</button>
		</section> -->
	</div>
{/if}

<svelte:window on:keydown={handleKeydown} />

<!-- <style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		background-color: rgba(0, 0, 0, 0.5);
		z-index: 50;
	}

	.modal-content {
		background: white;
		padding: 20px;
		border-radius: 8px;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
		position: relative;
		cursor: default;
	}

	.modal-title {
		margin-bottom: 20px;
	}

	.bubbles-container {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		margin-bottom: 20px;
	}

	.entity-bubble {
		background-color: #007bff;
		color: white;
		padding: 8px 16px;
		border-radius: 20px;
		white-space: nowrap;
	}

	.close-btn {
		padding: 6px 12px;
		background-color: #007bff;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	.close-btn:hover {
		background-color: #0056b3;
	}
</style> -->
