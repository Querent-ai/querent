<script lang="ts">
	import { createEventDispatcher, onMount } from 'svelte';
	import type {
		InsightInfo,
		CustomInsightOption,
		InsightCustomOptionValue
	} from '../../../../service/bindings';

	export let insightInfo: InsightInfo;
	export let show = false;

	const dispatch = createEventDispatcher();

	let formData: { [key: string]: string } = {};

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
		formData = Object.keys(insightInfo.additionalOptions).reduce(
			(acc, key) => {
				acc[key] = '';
				return acc;
			},
			{} as { [key: string]: string }
		);
	}
	let additionalOptionsEntries: [string, CustomInsightOption][];

	$: if (show) {
		additionalOptionsEntries = [];
		initializeFormData();

		additionalOptionsEntries = Object.entries(insightInfo.additionalOptions);

		if (insightInfo.id == 'querent.insights.graph_builder.gbv1') {
			if (!additionalOptionsEntries.some(([key]) => key === 'semantic_pipeline_id')) {
				const semanticPipelineIdOption: CustomInsightOption = {
					id: 'semantic_pipeline_id',
					label: 'Semantic Pipeline ID',
					tooltip: 'Enter the pipeline ID',
					value: {
						type: 'string',
						value: '',
						hidden: false
					}
				};
				additionalOptionsEntries.push(['semantic_pipeline_id', semanticPipelineIdOption]);
			}
			if (!additionalOptionsEntries.some(([key]) => key === 'query')) {
				const queryOption: CustomInsightOption = {
					id: 'query',
					label: 'Query',
					tooltip: 'Enter your query',
					value: {
						type: 'string',
						value: '',
						hidden: false
					}
				};
				additionalOptionsEntries.push(['query', queryOption]);
			}
		}
	}

	onMount(() => {
		initializeFormData();
	});
</script>

{#if show}
	<div class="modal-backdrop" role="dialog" aria-modal="true">
		<div class="modal">
			<div class="modal-header">
				<h2>Additional Insights for {insightInfo.name}</h2>
				<button class="close-button" on:click={closeModal} aria-label="Close modal">&times;</button>
			</div>
			<div class="modal-body">
				<form on:submit|preventDefault={submitForm}>
					{#each additionalOptionsEntries as [key, option]}
						<div class="form-group">
							<label for={key}>{key}</label>
							<input
								id={key}
								type="text"
								bind:value={formData[key]}
								placeholder={option.tooltip || ''}
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
		background-color: white;
		border-radius: 4px;
		width: 90%;
		max-width: 500px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #e0e0e0;
	}

	.modal-body {
		padding: 1rem;
	}

	.close-button {
		background: none;
		border: none;
		font-size: 1.5rem;
		cursor: pointer;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	label {
		display: block;
		margin-bottom: 0.5rem;
	}

	input {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ccc;
		border-radius: 4px;
	}

	.button-group {
		display: flex;
		justify-content: flex-end;
		gap: 1rem;
		margin-top: 1rem;
	}

	.cancel-button,
	.submit-button {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	.cancel-button {
		background-color: #f0f0f0;
	}

	.submit-button {
		background-color: #4caf50;
		color: white;
	}
</style>
