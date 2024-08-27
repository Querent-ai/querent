<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Button, Checkbox, Heading } from 'flowbite-svelte';
	import { Table, TableBody, TableBodyCell, TableBodyRow, TableHead } from 'flowbite-svelte';
	import { TableHeadCell, Toolbar } from 'flowbite-svelte';
	import MetaTag from '../../../utils/MetaTag.svelte';
	import { goto } from '$app/navigation';
	import { dataSources } from '../../../../stores/appState';
	import GoogleDriveIcon from './add/DriveComponent.svelte';
	import LocalStorageIcon from './add/FolderComponent.svelte';
	import DropboxIcon from './add/DropboxComponent.svelte';
	import AwsIcon from './add/AwsComponent.svelte';
	import AzureIcon from './add/AzureComponent.svelte';
	import GithubIcon from './add/GithubComponent.svelte';
	import OnedriveIcon from './add/OnedriveComponent.svelte';
	import JiraIcon from './add/JiraComponent.svelte';
	import SlackIcon from './add/SlackComponent.svelte';
	import EmailIcon from './add/EmailComponent.svelte';
	import NewsIcon from './add/NewsComponent.svelte';
	import GCSIcon from './add/GCSComponent.svelte';
	import { Trash } from 'svelte-bootstrap-icons';
	import {
		commands,
		type Backend,
		type ListCollectorConfig,
		type DeleteCollectorRequest,
		type CollectorConfig
	} from '../../../../service/bindings';
	import { onMount } from 'svelte';

	function navigateToAddNewSource() {
		goto('/crud/sources/add');
	}

	const path: string = '/crud/source';
	const description: string = 'Sources example - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - Sources';
	const subtitle: string = 'Sources';

	let sources_list: ListCollectorConfig = { config: [] };

	$: sources_list = { config: $dataSources };

	function getImage(backend: Backend | null): any {
		if (!backend) return '';
		if ('files' in backend) return LocalStorageIcon;
		if ('drive' in backend) return GoogleDriveIcon;
		if ('azure' in backend) return AzureIcon;
		if ('dropbox' in backend) return DropboxIcon;
		if ('email' in backend) return EmailIcon;
		if ('gcs' in backend) return GCSIcon;
		if ('github' in backend) return GithubIcon;
		if ('jira' in backend) return JiraIcon;
		if ('news' in backend) return NewsIcon;
		if ('onedrive' in backend) return OnedriveIcon;
		if ('s3' in backend) return AwsIcon;
		if ('slack' in backend) return SlackIcon;
		return null;
	}

	function getId(backend: Backend | null): string {
		if (!backend) return '';
		if ('azure' in backend) return backend.azure.id;
		if ('drive' in backend) return backend.drive.id;
		if ('files' in backend) return backend.files.id;
		if ('dropbox' in backend) return backend.dropbox.id;
		if ('email' in backend) return backend.email.id;
		if ('gcs' in backend) return backend.gcs.id;
		if ('github' in backend) return backend.github.id;
		if ('jira' in backend) return backend.jira.id;
		if ('news' in backend) return backend.news.id;
		if ('onedrive' in backend) return backend.onedrive.id;
		if ('s3' in backend) return backend.s3.id;
		if ('slack' in backend) return backend.slack.id;
		return '';
	}

	async function deleteSource(id: string) {
		try {
			let deleteRequest: DeleteCollectorRequest = {
				id: [id]
			};

			const deleteResult = await commands.deleteCollectors(deleteRequest);
			if (deleteResult) {
				let sources = await commands.getCollectors();
				$dataSources = sources.config;
			}
		} catch (error) {
			console.error('Error deleting source:', error);
			alert('Failed to delete source. Please try again.');
		}
	}

	onMount(async () => {
		try {
			if (sources_list.config.length === 0) {
				let sources = await commands.getCollectors();
				$dataSources = sources.config;
			}
		} catch (error) {
			console.error('Error fetching sources:', error);
			alert('Failed to load sources. Please try again.');
		}
	});
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem href="/crud/sources">Sources</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			All sources
		</Heading>

		<Toolbar embedded class="w-full py-4 text-gray-500 dark:text-gray-400">
			<div slot="end" class="space-x-2">
				<Button class="whitespace-nowrap" on:click={navigateToAddNewSource}>Add new source</Button>
			</div>
		</Toolbar>
	</div>
	<Table>
		<TableHead class="border-y border-gray-200 bg-gray-100 dark:border-gray-700">
			{#each ['Type', 'Name', 'ID'] as title}
				<TableHeadCell class="ps-4 font-normal">{title}</TableHeadCell>
			{/each}
			<TableHeadCell class="pe-100 ps-4 text-right font-normal">Delete</TableHeadCell>
		</TableHead>
		<TableBody>
			{#if Array.isArray(sources_list.config)}
				{#each sources_list.config as source}
					<TableBodyRow class="text-base">
						<TableBodyCell class="p-4">
							<svelte:component this={getImage(source.backend)} />
						</TableBodyCell>
						<TableBodyCell
							class="overflow-hidden truncate p-4 text-base font-normal text-gray-500 dark:text-gray-400"
							>{source.name}</TableBodyCell
						>
						<TableBodyCell class="flex items-center space-x-2 whitespace-nowrap p-4">
							{getId(source.backend)}
						</TableBodyCell>

						<TableBodyCell class="p-4 text-right">
							<Button color="red" size="xs" on:click={() => deleteSource(getId(source.backend))}>
								<Trash class="mr-2 h-4 w-4" />
								Delete
							</Button>
						</TableBodyCell>
					</TableBodyRow>
				{/each}
			{/if}
		</TableBody>
	</Table>
</main>
