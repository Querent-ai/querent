<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Button, Card, Heading } from 'flowbite-svelte';
	import { Input, Textarea, Toggle } from 'flowbite-svelte';
	import MetaTag from '../../../utils/MetaTag.svelte';

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
	let ingested_tokens = {
		file: '',
		data: [''],
		isTokenStream: false,
		docSource: '',
		sourceId: ''
	};

	function handleSubmit() {
		ingested_tokens.data = [formData.data];
		ingested_tokens.docSource = formData.doc_source;
		ingested_tokens.file = formData.file;
		ingested_tokens.isTokenStream = true;
		ingested_tokens.sourceId = formData.source_id;

		console.log('Form submitted:', ingested_tokens);
	}
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem href="/crud/sources">Sources</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			Realtime Sources
		</Heading>
	</div>

	<div class="flex justify-center">
		<Card class="m-4">
			<Heading tag="h2" class="mb-4 text-center text-lg font-semibold">Enter your data</Heading>
			<form on:submit|preventDefault={handleSubmit} class="space-y-4">
				<div>
					<Input
						type="text"
						label="Data"
						bind:value={formData.data}
						placeholder="Enter data string"
					/>
				</div>

				<div>
					<Input
						type="text"
						label="Doc Source"
						bind:value={formData.doc_source}
						placeholder="Enter doc source"
					/>
				</div>

				<div>
					<Input type="text" label="File" bind:value={formData.file} placeholder="Enter file" />
				</div>

				<div>
					<Input
						type="text"
						label="Source ID"
						bind:value={formData.source_id}
						placeholder="Enter source ID"
					/>
				</div>

				<Button type="submit">Submit</Button>
			</form>
		</Card>
	</div>
</main>
