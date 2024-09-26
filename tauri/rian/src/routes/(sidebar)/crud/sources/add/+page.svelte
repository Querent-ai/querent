<script lang="ts">
	import { Breadcrumb, BreadcrumbItem, Heading } from 'flowbite-svelte';
	import GCSForm from '../GCS.svelte';
	import AzureForm from '../Azure.svelte';
	import DriveForm from '../Drive.svelte';
	import DropboxForm from '../Dropbox.svelte';
	import EmailForm from '../Email.svelte';
	import GithubForm from '../Github.svelte';
	import JiraForm from '../Jira.svelte';
	import NewsForm from '../News.svelte';
	import AWSForm from '../S3.svelte';
	import SlackForm from '../Slack.svelte';
	import LocalStorageForm from '../LocalStorage.svelte';
	import GoogleDriveIcon from './DriveComponent.svelte';
	import FolderIcon from './FolderComponent.svelte';
	import DropboxIcon from './DropboxComponent.svelte';
	import AwsIcon from './AwsComponent.svelte';
	import AzureIcon from './AzureComponent.svelte';
	import GithubIcon from './GithubComponent.svelte';
	import OnedriveIcon from './OnedriveComponent.svelte';
	import JiraIcon from './JiraComponent.svelte';
	import SlackIcon from './SlackComponent.svelte';
	import EmailIcon from './EmailComponent.svelte';
	import NewsIcon from './NewsComponent.svelte';
	import GCSIcon from './GCSComponent.svelte';
	import MetaTag from '../../../../utils/MetaTag.svelte';
	import Modal from './Modal.svelte';
	import { googleDriveRefreshToken, isVisible } from '../../../../../stores/appState';
	import { open } from '@tauri-apps/plugin-shell';

	import { commands } from '../../../../../service/bindings';

	async function get_drive_client_secrets() {
		const result = await commands.getDriveCredentials();
		if (result.status == 'ok') {
			CLIENT_ID = result.data[0];
			REDIRECT_URI = 'http://localhost:5174/confirmation';
		}
	}
	let CLIENT_ID: string;
	let REDIRECT_URI: string;

	async function login() {
		try {
			await get_drive_client_secrets();

			const AUTH_URL = `https://accounts.google.com/o/oauth2/auth?client_id=${CLIENT_ID}&redirect_uri=${encodeURIComponent(REDIRECT_URI)}&response_type=code&scope=https://www.googleapis.com/auth/drive.readonly&access_type=offline`;
			open(AUTH_URL);

			const res = await getDriveCode();

			if (res) {
				await getTokens(res);
				selectedSource = 'Google Drive';
				setIsVisible();
			}
		} catch (error) {
			console.log('Error starting the OAuth server  ', error);
		}
	}

	async function getDriveCode(): Promise<string> {
		try {
			const code = await commands.startOauthServer();
			if (code.status == 'ok') {
				return code.data;
			} else {
				throw new Error('No code received');
			}
		} catch (error) {
			console.error('Error fetching Google Drive code: ', error);
			throw error;
		}
	}

	let drive_client_id: string;
	let drive_client_secret: string;
	async function getTokens(code: string) {
		const result = await commands.getDriveCredentials();
		if (result.status == 'ok') {
			drive_client_id = result.data[0];
			drive_client_secret = result.data[1];
		}
		const redirect_uri = 'http://localhost:5174/confirmation';
		const response = await fetch('https://oauth2.googleapis.com/token', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/x-www-form-urlencoded'
			},
			body: new URLSearchParams({
				code: code,
				client_id: drive_client_id,
				client_secret: drive_client_secret,
				redirect_uri: redirect_uri,
				grant_type: 'authorization_code'
			})
		});

		if (!response.ok) {
			console.error('HTTP error! status:', response.status);
			console.log('Error response body:', await response.text());
			return;
		}
		const data = await response.json();
		googleDriveRefreshToken.set(data.refresh_token);
	}

	function getIcon(sourceName: string) {
		return iconMapping[sourceName as keyof typeof iconMapping];
	}

	let selectedSource: string | null = null;

	let configurations: Record<string, any> = {
		'Local Storage': { 'Storage Path': '' },
		'Google Drive': { 'Drive ID': '', Credentials: '' },
		'Google Cloud Storage': { 'Bucket Name': '', Credentials: '' },
		Azure: { 'Connection URL': '', 'Account URL': '', Credentials: '', Container: '', Prefix: '' },

		Dropbox: {
			'Dropbox App Key': '',
			'Dropbox Refresh Token': '',
			'Dropbox App Secret': '',
			'Dropbox Folder Path': ''
		},
		Email: {
			'IMAP Server': '',
			Port: '',
			Username: '',
			Password: '',
			Folder: '',
			'Certificate File (optional)': '',
			'Key File (optional)': ''
		},
		Github: { 'GitHub Username': '', 'Repository Name': '', 'Access Token': '' },
		Jira: {
			'Jira Server URL': '',
			'Jira Username': '',
			'Jira API Token': '',
			'Jira Project': '',
			'Jira Query': ''
		},
		News: {
			'API Key': '',
			'Query Topic': '',
			'Date Range From': '',
			'Date Range To': '',
			Language: '',
			'Sort By': '',
			'Page Size': '',
			'Page Number': ''
		},
		'AWS S3': { Bucket: '', Region: '', 'Access Key': '', 'Secret Key': '' },
		Slack: {
			'Channel Name': '',
			'Cursor (optional)': '',
			'Include All Metadata': '',
			'Inclusive (Include messages with latest or oldest timestamp)': '',
			'Latest (timestamp)': '',
			Limit: '',
			'Access Token': ''
		},
		Onedrive: {}
	};

	let premiumSources = [
		'Azure',
		'Dropbox',
		'Email',
		'Github',
		'Jira',
		'News',
		'AWS S3',
		'Slack',
		'Google Cloud Storage',
		'Google Drive'
	];

	let showModal = false;
	let modalMessage = '';

	function selectSource(sourceName: string) {
		if (premiumSources.includes(sourceName)) {
			modalMessage = 'This feature is available only in premium';
			showModal = true;
		} else if (sourceName === 'Google Drive') {
			login().catch(console.error);
		} else {
			$isVisible = true;
			selectedSource = sourceName;
		}
	}

	function setIsVisible(): string {
		$isVisible = true;
		return '';
	}

	const iconMapping: Record<string, any> = {
		'Google Drive': GoogleDriveIcon,
		'Local Storage': FolderIcon,
		'Google Cloud Storage': GCSIcon,
		Azure: AzureIcon,
		Dropbox: DropboxIcon,
		Email: EmailIcon,
		Github: GithubIcon,
		Jira: JiraIcon,
		News: NewsIcon,
		'AWS S3': AwsIcon,
		Slack: SlackIcon,
		Onedrive: OnedriveIcon
	};

	const path: string = '/crud/sources/add';
	const description: string = 'Add new source - Querent Admin Dashboard';
	const title: string = 'Querent Admin Dashboard - Add New Source';
	const subtitle: string = 'Add New Source';
</script>

<MetaTag {path} {description} {title} {subtitle} />

<main class="relative h-full w-full overflow-y-auto bg-white dark:bg-gray-800">
	<div class="p-4">
		<Breadcrumb class="mb-5">
			<BreadcrumbItem home>Home</BreadcrumbItem>
			<BreadcrumbItem href="/crud/sources">Sources</BreadcrumbItem>
			<BreadcrumbItem>Add New Source</BreadcrumbItem>
		</Breadcrumb>
		<Heading tag="h1" class="text-xl font-semibold text-gray-900 dark:text-white sm:text-2xl">
			List of Sources
		</Heading>
		<div class="mt-6 grid grid-cols-2 gap-6 sm:grid-cols-3 lg:grid-cols-4">
			{#each Object.keys(configurations) as sourceName}
				<button
					type="button"
					class="flex cursor-pointer flex-col items-center space-y-2"
					on:click={() => selectSource(sourceName)}
					on:keydown={(event) => event.key === 'Enter' && (selectedSource = sourceName)}
					aria-label={`Select ${sourceName}`}
				>
					<svelte:component this={getIcon(sourceName)} class="h-16 w-16" />
					<span class="text-center text-gray-700 dark:text-gray-200">{sourceName}</span>
				</button>
			{/each}
		</div>
		{#if selectedSource === 'Google Cloud Storage'}
			<GCSForm configuration={configurations['Google Cloud Storage']} />
		{:else if selectedSource === 'Azure'}
			<AzureForm configuration={configurations['Azure']} />
		{:else if selectedSource === 'Google Drive'}
			{setIsVisible()}
			<DriveForm />
		{:else if selectedSource === 'Dropbox'}
			<DropboxForm configuration={configurations['Dropbox']} />
		{:else if selectedSource === 'Email'}
			<EmailForm configuration={configurations['Email']} />
		{:else if selectedSource === 'Github'}
			<GithubForm configuration={configurations['Github']} />
		{:else if selectedSource === 'Jira'}
			<JiraForm configuration={configurations['Jira']} />
		{:else if selectedSource === 'News'}
			<NewsForm configuration={configurations['News']} />
		{:else if selectedSource === 'AWS S3'}
			<AWSForm configuration={configurations['AWS S3']} />
		{:else if selectedSource === 'Slack'}
			<SlackForm configuration={configurations['Slack']} />
		{:else if selectedSource === 'Local Storage'}
			<LocalStorageForm />
		{/if}
		<Modal bind:show={showModal} message={modalMessage} />
	</div>
</main>
