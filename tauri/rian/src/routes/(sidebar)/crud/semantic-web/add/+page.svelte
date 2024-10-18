<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import AGNForm from './AGN.svelte';
	import AgnIcon from './AGNIcon.svelte';
	import CodeFabricIcon from './CodeFabricIcon.svelte';
	import Modal from '../../sources/add/Modal.svelte';
	import TimeSeriesFabric from './TimeSeriesFabric.svelte';
	import GeoFabricIcon from './GeoFabricIcon.svelte';
	export let formOpen = true;

	let activeForm: string | number | null = null;

	const formsList = [
		{
			name: 'Attention',
			form: AGNForm,
			icon: AgnIcon,
			description: 'Attention Graph Fabric',
			tooltip:
				'Attention Graph Fabric is a semantic engine that weaves concepts into dynamic knowledge networks using advanced language processing and graph theory.'
		},
		{
			name: 'Temporal',
			form: null,
			icon: TimeSeriesFabric,
			description: 'Temporal Graph Fabric',
			tooltip:
				'Temporal Graph Fabric is a data processing engine that weaves temporal patterns into a rich, interconnected network for advanced forecasting and trend analysis.'
		},
		{
			name: 'Geo',
			form: null,
			icon: GeoFabricIcon,
			description: 'Geo Graph Fabric',
			tooltip:
				'Geo Graph Fabric is a data processing engine that helps in identifying geo locations.'
		},
		{
			name: 'Code',
			form: null,
			icon: CodeFabricIcon,
			description: 'Code Graph Fabric',
			tooltip:
				'Code Graph Fabric is a software analysis engine that constructs interconnected representations of codebases, enabling deep insights and intelligent navigation across complex software systems.'
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
			modalMessage = 'This feature is available only in premium';
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
			Available Data Fabrics
		</Heading>

		<div class="mt-6 flex flex-wrap justify-start gap-8">
			{#each formsList as form (form.name)}
				<div class="card-container">
					<button
						type="button"
						class="flex w-full cursor-pointer items-start space-x-4 rounded-lg p-4 transition-colors hover:bg-gray-100 dark:hover:bg-gray-800"
						on:click={() => setActiveForm(form.name)}
						aria-label={`Select ${form.name}`}
						title={form.tooltip}
					>
						<svelte:component this={form.icon} />
						<div class="text-left">
							<span class="text-left text-lg text-gray-700 dark:text-gray-200">
								{form.name}
								{#if !form.form}
									<!-- Styled Pro badge -->
									<span class="pro-badge">Pro</span>
								{/if}
							</span>
							<div class="text-sm text-gray-500 dark:text-gray-400">{form.description}</div>
						</div>
					</button>
				</div>
			{/each}
		</div>
		{#if activeForm}
			<svelte:component this={getFormComponent()} {formOpen} on:close={handleFormClose} />
		{/if}
	</div>
	<Modal bind:show={showModal} message={modalMessage} />
</main>

<style>
	.card-container {
		width: 250px; /* Set equal width for all cards */
		background-color: white; /* Optional card background color */
		border-radius: 0.5rem;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		overflow: hidden;
	}
	.card-container svg {
		width: 40px; /* Fixed width for icons */
		height: 40px; /* Fixed height for icons */
	}
	.pro-badge {
		background-color: #f97316; /* Orange color */
		color: white;
		padding: 2px 6px;
		border-radius: 0.25rem;
		font-size: 0.75rem;
		font-weight: 500;
		margin-left: 8px;
	}
</style>

