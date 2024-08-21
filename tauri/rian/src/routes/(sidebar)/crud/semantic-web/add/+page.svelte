<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import AGNForm from './AGN.svelte';
	import AgnIcon from './AGNIcon.svelte';
	import CodeFabricIcon from './CodeFabricIcon.svelte';
	import Modal from '../../sources/add/Modal.svelte';
	import TimeSeriesFabric from './TimeSeriesFabric.svelte';
	export let formOpen = true;

	let activeForm: string | number | null = null;

	const formsList = [
		{
			name: 'Attention',
			form: AGNForm,
			icon: AgnIcon,
			description: 'Attention Graph Fabric'
		},
		{
			name: 'Code',
			form: null,
			icon: CodeFabricIcon,
			description: 'Code Graph Fabric'
		},
		{
			name: 'TimeSeries',
			form: null,
			icon: TimeSeriesFabric,
			description: 'Time Series Fabric'
		}
	];

	function setActiveForm(formName: string) {
		activeForm = formName === activeForm ? null : formName;
		formOpen = activeForm !== null;
	}

	let showModal = false;
	let modalMessage = '';

	function getFormComponent() {
		const selectedForm = formsList.find((form) => form.name == activeForm);
		if (selectedForm && selectedForm.form) {
			return selectedForm.form;
		} else {
			showModal = true;
			modalMessage = 'This engine is available only in premium';
			activeForm = null;
		}
		return;
	}

	function handleFormClose() {
		activeForm = null;
		formOpen = false;
	}
</script>

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem href="/crud/semantic-web">Pipelines</BreadcrumbItem>
			<BreadcrumbItem>Start New Pipeline</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			Available Data Fabric
		</Heading>

		<div class="mt-6 flex flex-wrap justify-start gap-8">
			{#each formsList as form (form.name)}
				<button
					type="button"
					class="flex cursor-pointer items-start space-x-4 rounded-lg p-4 transition-colors hover:bg-gray-100 dark:hover:bg-gray-800"
					on:click={() => setActiveForm(form.name)}
					aria-label={`Select ${form.name}`}
				>
					<svelte:component this={form.icon} />
					<div class="text-left">
						<span class="text-left text-lg text-gray-700 dark:text-gray-200">{form.name}</span>
						<div class="text-sm text-gray-500 dark:text-gray-400">{form.description}</div>
					</div>
				</button>
			{/each}
		</div>
		{#if activeForm}
			<svelte:component this={getFormComponent()} {formOpen} on:close={handleFormClose} />
		{/if}
	</div>
	<Modal bind:show={showModal} message={modalMessage} />
</main>
