<script lang="ts">
	import { dataSources, updatePipeline, pipelineState } from '../../../../stores/appState';
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
	import { commands } from '../../../../service/bindings';
	import { onMount } from 'svelte';

	let pastPipelines;

	onMount(async() => {
		let pastAgns = await commands.getPastAgns();
		if (pastAgns.status == "ok") {
			pastPipelines = pastAgns.data;
		}

		let currentAgs = await commands.getRunningAgns();
		if (currentAgs.length == 0) {
			return
		}
		let currentPipelineID = currentAgs[0];
		let currentPipelineData = currentAgs[1];
		})

	let pipelines_list: any[] = [12];

	function navigateToStartPipeline() {
		goto('/crud/semantic-web/add');
	}
</script>

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem href="/crud/semantic-web">Data Fabric</BreadcrumbItem>
			<BreadcrumbItem>Engines</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			All Engines
		</Heading>

		<Toolbar embedded class="w-full py-4 text-gray-500 dark:text-gray-400">
			<div slot="end" class="space-x-2">
				<Button class="whitespace-nowrap" on:click={navigateToStartPipeline}
					>Start new pipeline</Button
				>
			</div>
		</Toolbar>
	</div>
	<Table>
		<TableHead class="border-y border-gray-200 bg-gray-100 dark:border-gray-700">
			{#each ['Type', 'ID', 'Fixed Entities', 'Sample Entities', 'Sources'] as title}
				<TableHeadCell class="ps-4 font-normal">{title}</TableHeadCell>
			{/each}
			<TableHeadCell class="pe-100 ps-4 text-right font-normal">Status</TableHeadCell>
		</TableHead>
		<TableBody>
			{#if Array.isArray(pipelines_list)}
				{#each pipelines_list as source}
					<TableBodyRow class="text-base">
						<TableBodyCell class="flex items-center space-x-2 whitespace-nowrap p-4"
						></TableBodyCell>
					</TableBodyRow>
				{/each}
			{/if}
		</TableBody>
	</Table>
</main>
