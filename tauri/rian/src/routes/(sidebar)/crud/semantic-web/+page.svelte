<script lang="ts">
	import {
		Breadcrumb,
		BreadcrumbItem,
		Button,
		Heading,
		Table,
		TableBody,
		TableBodyCell,
		TableBodyRow,
		TableHead,
		TableHeadCell,
		Toolbar
	} from 'flowbite-svelte';
	import { goto } from '$app/navigation';
	import { commands, type SemanticPipelineRequest } from '../../../../service/bindings';
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';

	interface SemanticPipelineData {
		collectors: string[];
		fixed_entities: string[] | undefined;
		sample_entities: string[] | undefined;
		model: number | null;
		status: string;
		id: string;
	}

	let pastPipelines;
	let pipelines_list = writable<SemanticPipelineData[]>([]);

	onMount(async () => {
		const result = await commands.getRunningAgns();

		result.forEach((runningAgns) => {
			const pipelineId = runningAgns[0];
			const pipelineInfo = runningAgns[1];
			let pipelineData: SemanticPipelineData = {
				collectors: pipelineInfo.collectors,
				fixed_entities: pipelineInfo.fixed_entities?.entities,
				sample_entities: pipelineInfo.sample_entities?.entities,
				model: pipelineInfo.model,
				status: 'active',
				id: pipelineId
			};

			pipelines_list.update((list) => [...list, pipelineData]);
		});

		let pastAgns = await commands.getPastAgns();
		if (pastAgns.status == 'ok') {
			pastPipelines = pastAgns.data.requests;

			pastPipelines.forEach((pastAgns) => {
				const pipelineId = pastAgns.pipeline_id;
				const pipelineInfo = pastAgns.request;
				if (!pipelineInfo) {
					return;
				}

				const isAlreadyAdded = result.some((runningAgn) => pipelineId == runningAgn[0]);

				if (!isAlreadyAdded) {
					let pipelineData: SemanticPipelineData = {
						collectors: pipelineInfo.collectors,
						fixed_entities: pipelineInfo.fixed_entities?.entities,
						sample_entities: pipelineInfo.sample_entities?.entities,
						model: pipelineInfo.model,
						status: 'stopped',
						id: pipelineId
					};

					pipelines_list.update((list) => [...list, pipelineData]);
				}
			});
		}
	});

	async function handleStopAgn(pipelineId: string) {
		let res = await commands.stopAgnFabric(pipelineId);
		if (res.status == 'error') {
			console.log('Error while stopping the pipeline ', res.error);
			return;
		}
		pipelines_list.update((list) =>
			list.map((pipeline) =>
				pipeline.id === pipelineId ? { ...pipeline, status: 'stopped' } : pipeline
			)
		);
	}

	function navigateToStartPipeline() {
		goto('/crud/semantic-web/add/');
	}
</script>

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem>Data Fabric</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			All Data Fabrics
		</Heading>

		<Toolbar embedded class="w-full py-4 text-gray-500 dark:text-gray-400">
			<div slot="end" class="space-x-2">
				<Button class="whitespace-nowrap" on:click={navigateToStartPipeline}
					>Start new pipeline</Button
				>
			</div>
		</Toolbar>
	</div>
	<Table class="w-full">
		<TableHead class="border-y border-gray-200 bg-gray-100 dark:border-gray-700">
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'Type'}</TableHeadCell>
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'ID'}</TableHeadCell>
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'Fixed entities'}</TableHeadCell>
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'Sample entities'}</TableHeadCell>
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'Source'}</TableHeadCell>
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'Status'}</TableHeadCell>
			<TableHeadCell class="w-1/6 px-4 py-2 font-normal">{'Action'}</TableHeadCell>
		</TableHead>
		<TableBody>
			{#if Array.isArray($pipelines_list)}
				{#each $pipelines_list as pipeline}
					<TableBodyRow class="text-base">
						<TableBodyCell class="px-4 py-2">
							<div class="break-words">{'AGN'}</div>
						</TableBodyCell>
						<TableBodyCell class="px-4 py-2">
							<div class="break-words">{pipeline.id}</div>
						</TableBodyCell>
						<TableBodyCell class="px-4 py-2">
							<div class="break-words">
								{#if pipeline.fixed_entities}
									<details class="dropdown">
										<summary class="cursor-pointer text-blue-500">Show Fixed Entities</summary>
										<div class="bubble-container">
											{#each pipeline.fixed_entities as entity}
												<span class="bubble">{entity}</span>
											{/each}
										</div>
									</details>
								{/if}
							</div>
						</TableBodyCell>
						<TableBodyCell class="px-4 py-2">
							<div class="break-words">
								{#if pipeline.sample_entities}
									<details class="dropdown">
										<summary class="cursor-pointer text-blue-500">Show Sample Entities</summary>
										<div class="bubble-container">
											{#each pipeline.sample_entities as entity}
												<span class="bubble">{entity}</span>
											{/each}
										</div>
									</details>
								{/if}
							</div>
						</TableBodyCell>
						<TableBodyCell class="px-4 py-2">
							<div class="break-words">
								{#if pipeline.collectors}
									<details class="dropdown">
										<summary class="cursor-pointer text-blue-500">Show Sources</summary>
										<div class="bubble-container">
											{#each pipeline.collectors as entity}
												<span class="bubble">{entity}</span>
											{/each}
										</div>
									</details>
								{/if}
							</div>
						</TableBodyCell>
						<TableBodyCell class="px-4 py-2">
							<div class="break-words">{pipeline.status}</div>
						</TableBodyCell>
						<TableBodyCell class="px-4 py-2">
							{#if pipeline.status == 'active'}
								<button
									class="rounded bg-blue-500 px-4 py-2 font-bold text-white hover:bg-blue-700"
									on:click={() => handleStopAgn(pipeline.id)}
								>
									Stop
								</button>
							{/if}
						</TableBodyCell>
					</TableBodyRow>
				{/each}
			{/if}
		</TableBody>
	</Table>
</main>

<style>
	.bubble-container {
		margin-top: 0.5rem;
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.bubble {
		background-color: #e2e8f0;
		padding: 0.25rem 0.75rem;
		border-radius: 9999px;
		display: inline-block;
		font-size: 0.875rem;
	}
	.dropdown summary {
		outline: none;
	}
</style>
