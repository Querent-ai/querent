<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import type { InsightInfo, CustomInsightOption } from '../../../../service/bindings';

	export let insightInfo: InsightInfo;
	export let show = false;

	const dispatch = createEventDispatcher();

	let formData: { [key: string]: string } = {};
	let additionalOptionsEntries: [string, CustomInsightOption][] = [];

	function closeModal() {
		show = false;
		dispatch('close');
	}

	function submitForm() {
		let res: { [key: string]: CustomInsightOption } = {};

		for (const key in formData) {
			const value = formData[key];

			res[key] = {
				id: key,
				label: key,
				tooltip: null,
				value: { type: 'string', value: value, hidden: null }
			};
		}
		dispatch('submit', res);
		closeModal();
	}

	function initializeFormData() {
		// Initialize formData with empty strings for all additional options
		formData = Object.keys(insightInfo.additionalOptions).reduce(
			(acc, key) => {
				acc[key] = '';
				return acc;
			},
			{} as { [key: string]: string }
		);

		// Prepare the entries for displaying in the form
		additionalOptionsEntries = Object.entries(insightInfo.additionalOptions);
	}

	// Helper function to convert snake_case to Title Case
	function toTitleCase(str: string): string {
		return str
			.split('_')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1))
			.join(' ');
	}

	// Reactively watch the 'show' property to reinitialize when the modal opens
	$: if (show) {
		initializeFormData();
	}

	// Also reinitialize when the 'insightInfo' changes to handle different insights
	$: if (insightInfo) {
		initializeFormData();
	}

	onMount(() => {
		initializeFormData();
	});
</script>

{#if show}
	<div class="modal-backdrop" role="dialog" aria-modal="true">
		<div class="modal">
			<div class="modal-header">
				<h2>Configure {insightInfo.name}</h2>
				<button class="close-button" on:click={closeModal} aria-label="Close modal">&times;</button>
			</div>
			<div class="modal-body">
				<form on:submit|preventDefault={submitForm}>
					{#each additionalOptionsEntries as [key, option]}
						<div class="form-group">
							<label for={key}>
								{toTitleCase(key)}
								{#if option.tooltip}
									<span class="info-icon" title={option.tooltip} aria-label={option.tooltip}>
										ℹ️
									</span>
								{/if}
							</label>
							<input
								id={key}
								type="text"
								bind:value={formData[key]}
								placeholder=""
								disabled={key === 'prompt'}
							/>
						</div>
					{/each}

					<div class="button-group">
						<button type="button" class="cancel-button" on:click={closeModal}>Cancel</button>
						<button type="submit" class="submit-button">Submit</button>
					</div>
				</form>
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
		z-index: 1000;
	}

	.modal {
		background-color: #ffffff;
		border-radius: 8px;
		width: 90%;
		max-width: 400px;
		box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
		overflow: hidden;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background-color: #f0f0f0;
		border-bottom: 1px solid #ddd;
	}

	.modal-header h2 {
		font-size: 1.25rem;
		margin: 0;
		color: #333;
	}

	.close-button {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
		color: #999;
	}

	.close-button:hover {
		color: #666;
	}

	.modal-body {
		padding: 1rem;
		background-color: #fafafa;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: flex;
		align-items: center;
		margin-bottom: 0.5rem;
		color: #555;
		font-size: 0.875rem;
	}

	.info-icon {
		margin-left: 0.5rem;
		cursor: pointer;
		font-size: 0.9rem;
		color: #888;
	}

	.info-icon:hover {
		color: #555;
	}

	.form-group input {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ccc;
		border-radius: 4px;
		font-size: 0.95rem;
	}

	.button-group {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 1rem;
	}

	.cancel-button,
	.submit-button {
		padding: 0.5rem 1rem;
		border-radius: 4px;
		cursor: pointer;
		border: none;
		font-size: 0.875rem;
	}

	.cancel-button {
		background-color: #e0e0e0;
		color: #333;
	}

	.submit-button {
		background-color: #4caf50;
		color: #fff;
	}

	.submit-button:hover {
		background-color: #45a049;
	}

	.cancel-button:hover {
		background-color: #d5d5d5;
	}
</style>
