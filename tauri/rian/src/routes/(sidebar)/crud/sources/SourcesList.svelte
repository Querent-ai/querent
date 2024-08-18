<script lang="ts">
	import { createEventDispatcher, SvelteComponent } from 'svelte';
	const dispatch = createEventDispatcher();

	function handleSave(event: {
		detail: {
			name: string;
			technology: string;
			description: string;
			configuration: Record<string, unknown>;
		};
	}) {
		dispatch('sourceSaved', {
			name: event.detail.name,
			technology: event.detail.technology,
			description: event.detail.description,
			configuration: event.detail.configuration
		});
		console.log('Sending data in collectors list as ', event.detail.configuration);
	}

	export let configuration: Record<string, string>;
	import DriveForm from './Drive.svelte';
	import AzureForm from './Azure.svelte';
	import DropboxForm from './Dropbox.svelte';
	import EmailForm from './Email.svelte';
	import GCSForm from './GCS.svelte';
	import GithubForm from './Github.svelte';
	import JiraForm from './Jira.svelte';
	import S3Form from './S3.svelte';
	import SlackForm from './Slack.svelte';
	import NewsForm from './News.svelte';
	import SourceComponent from '../../../data/source-component.json';
	import SourceComponentPremium from '../../../data/source-component-premium.json';
	import LocalStorageForm from './LocalStorage.svelte';

	let selectedSource: string | null = null;

	interface FormComponents {
		[key: string]: typeof SvelteComponent<any, any, any>;
	}

	const formComponents: FormComponents = {
		AWSForm: S3Form,
		AzureForm: AzureForm,
		DriveForm: DriveForm,
		DropboxForm: DropboxForm,
		EmailForm: EmailForm,
		GCSForm: GCSForm,
		GithubForm: GithubForm,
		JiraForm: JiraForm,
		NewsForm: NewsForm,
		SlackForm: SlackForm,
		LocalStorageForm: LocalStorageForm
	};

	function getFormComponent(name: string) {
		const entry = [...SourceComponent, ...SourceComponentPremium].find((c) => c.name === name);
		return entry ? formComponents[entry.formComponent] : null;
	}

	function isPremiumSource(name: string) {
		return SourceComponentPremium.some((source) => source.name === name);
	}
</script>

<select bind:value={selectedSource}>
	<option disabled selected value={null}>-- Select a source --</option>
	{#each SourceComponent as source}
		<option value={source.name}>{source.name}</option>
	{/each}
	{#each SourceComponentPremium as source}
		<option value={source.name} style="color: #888;">{source.name}</option>
	{/each}
</select>

{#if selectedSource}
	<div class="form-container">
		{#if isPremiumSource(selectedSource)}
			<p>This feature is available only in premium.</p>
		{:else if getFormComponent(selectedSource)}
			<svelte:component this={getFormComponent(selectedSource)} on:sourceSaved={handleSave} />
		{/if}
	</div>
{/if}

<style>
	.form-container {
		margin-top: 1rem;
		padding: 1rem;
		border: 1px solid #ccc;
		border-radius: 8px;
	}
</style>
