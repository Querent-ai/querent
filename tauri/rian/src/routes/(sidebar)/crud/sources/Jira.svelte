<script lang="ts">
	import { Button, Heading, Input, Label, Textarea } from 'flowbite-svelte';

	import { createEventDispatcher, onMount } from 'svelte';
	const dispatch = createEventDispatcher();

	export let sourceName: string = 'Jira';

	let jira_server: string = '';
	let jira_username: string = '';
	let jira_api_token: string = '';
	let jira_project: string = '';
	let jira_query: string = '';

	export let configuration: {
		'Jira Server URL': string;
		'Jira Username': string;
		'Jira API Token': string;
		'Jira Project': string;
		'Jira Query': string;
	};
	function initializeForm() {
		if (configuration) {
			jira_server = configuration['Jira Server URL'] || '';
			jira_username = configuration['Jira Username'] || '';
			jira_api_token = configuration['Jira API Token'] || '';
			jira_project = configuration['Jira Project'] || '';
			jira_query = configuration['Jira Query'] || '';
		}
	}

	onMount(() => {
		initializeForm();
	});

	$: if (configuration) {
		initializeForm();
	}

	function saveConfiguration() {
		dispatch('saveSource', {
			name: sourceName,
			technology: 'Jira',
			description: 'Configured Jira source',
			configuration: {
				'Jira Server URL': jira_server,
				'Jira Username': jira_username,
				'Jira API Token': jira_api_token,
				'Jira Project': jira_project,
				'Jira Query': jira_query
			}
		});
	}
</script>

<Heading tag="h5" class="mb-6 text-sm font-semibold uppercase">Configure Jira Integration</Heading>

<form on:submit|preventDefault={saveConfiguration}>
	<div class="space-y-4">
		<Label class="space-y-2">
			<span>Jira Server URL</span>
			<Input
				bind:value={jira_server}
				class="border font-normal outline-none"
				placeholder="Enter your Jira server URL"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Jira Username</span>
			<Input
				bind:value={jira_username}
				class="border font-normal outline-none"
				placeholder="Enter your Jira username"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Jira API Token</span>
			<Input
				bind:value={jira_api_token}
				type="password"
				class="border font-normal outline-none"
				placeholder="Paste your Jira API token"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Jira Project</span>
			<Input
				bind:value={jira_project}
				class="border font-normal outline-none"
				placeholder="Enter your Jira project key"
				required
			/>
		</Label>

		<Label class="space-y-2">
			<span>Jira Query</span>
			<Textarea
				bind:value={jira_query}
				class="border font-normal outline-none"
				placeholder="Enter your Jira query"
				rows="3"
				required
			></Textarea>
		</Label>

		<div class="flex w-full justify-center space-x-4 pb-4">
			<Button type="submit" class="w-full">Save Configuration</Button>
		</div>
	</div>
</form>
