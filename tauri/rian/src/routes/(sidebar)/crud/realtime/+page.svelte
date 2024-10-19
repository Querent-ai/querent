<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Button, Card, Heading } from 'flowbite-svelte';
	import { Input } from 'flowbite-svelte';
	import MetaTag from '../../../utils/MetaTag.svelte';
	import { commands, type IngestedTokens } from '../../../../service/bindings';
	import { pipelineState } from '../../../../stores/appState';

	let hidden: boolean = true;

	const path: string = '/crud/sources';
	const description: string = 'CRUD products example - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - CRUD Products';
	const subtitle: string = 'CRUD Products';

	let formData = {
		data: '',
		doc_source: '',
		file: '',
		is_token_stream: false,
		source_id: ''
	};
	let ingested_tokens: IngestedTokens = {
		file: '',
		data: [''],
		doc_source: '',
		source_id: '',
		is_token_stream: true,
		image_id: ''
	};

	async function handleSubmit() {
		try {
			ingested_tokens.data = [formData.data];
			ingested_tokens.doc_source = formData.doc_source;
			ingested_tokens.file = formData.file;
			ingested_tokens.is_token_stream = true;
			ingested_tokens.source_id = formData.source_id;

			const response = await commands.ingestTokens([ingested_tokens], $pipelineState.id);
			if (response) {
				console.log('Data submitted successfully.');
			} else {
				console.error('Failed to submit data.');
			}
		} catch (error) {
			console.error('An error occurred while submitting data:', error);
		}
	}
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="main-container">
	<div class="breadcrumb-container">
		<div class="p-4">
			<Breadcrumb class="mb-5">
				<BreadcrumbItem home>Home</BreadcrumbItem>
				<BreadcrumbItem href="/crud/sources">Sources</BreadcrumbItem>
			</Breadcrumb>
			<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
				Realtime Sources
			</Heading>
		</div>
	</div>

	<div class="card-container">
		<Card
			style="max-width: 600px; width: 100%; border: 0px transparent; transition: border-color 0.3s ease; color: black; padding-top: 150px; padding-bottom: 150px; font-size: 24px;"
			aria-label="Premium feature information"
		>
			This feature is only available in the premium
		</Card>
	</div>

	<!-- {#if $pipelineState.id}
		<div class="flex justify-center">
			<Card class="m-4">
				<Heading tag="h2" class="mb-4 text-center text-lg font-semibold">Enter your data</Heading>
				<form on:submit|preventDefault={handleSubmit} class="space-y-4">
					<div>
						<Input type="text" label="Data" bind:value={formData.data} placeholder="Enter data" />
					</div>

					<div>
						<Input
							type="text"
							label="Doc Source"
							bind:value={formData.doc_source}
							placeholder="Enter data source"
						/>
					</div>

					<div>
						<Input
							type="text"
							label="File"
							bind:value={formData.file}
							placeholder="Enter file name"
						/>
					</div>

					<div>
						<Input
							type="text"
							label="Source ID"
							bind:value={formData.source_id}
							placeholder="Enter source name"
						/>
					</div>

					<Button type="submit">Submit</Button>
				</form>
			</Card>
		</div>
	{/if} -->
</main>

<style>
	.main-container {
		position: relative;
		height: 100%;
		width: 100%;
		overflow-y: auto;
		background-color: white;
	}

	:global(.dark) .main-container {
		background-color: #1f2937;
	}

	.breadcrumb-container {
		padding: 1rem;
	}

	.card-container {
		display: flex;
		justify-content: center;
		align-items: center;
		height: calc(100% - 150px);
	}
</style>
