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
	import { onMount } from 'svelte';
	import Modal from './Modal.svelte';
	import { isVisible } from '../../../../../stores/appState';
	import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
	import { app } from '@tauri-apps/api';

	import { listen } from '@tauri-apps/api/event';
	import { invoke } from '@tauri-apps/api/core';

	listen('tauri://protocol-request', (event) => {
		const url = new URL(event.payload as string);
		const code = url.searchParams.get('code');
		if (code) {
			console.log('reached inside this whatever function');
		}
	});

	const CLIENT_ID = '4402204563-bmfpspke6cl23j2975hd7dkuf2v4ii3n.apps.googleusercontent.com';
	const REDIRECT_URI = 'http://localhost:5173/crud/sources/add';
	const AUTH_URL = `https://accounts.google.com/o/oauth2/auth?client_id=${CLIENT_ID}&redirect_uri=${encodeURIComponent(REDIRECT_URI)}&response_type=code&scope=https://www.googleapis.com/auth/drive&access_type=offline`;

	async function login() {
		// // opening a new window of the app itself
		// const authWindow = new WebviewWindow('authWindow', {
		// 	url: AUTH_URL,
		// });

		// console.log("Url is ", AUTH_URL)

		// authWindow.once('tauri://created', () => {
		// 	console.log('Authentication window created');
		// });

		// authWindow.once('tauri://error', (error) => {
		// 	console.error('Error loading authentication window:', error);
		// });

		// // shell working, opening in the browser
		// open(AUTH_URL);

		// const REDIRECT_URI = 'urn:ietf:wg:oauth:2.0:oob';
		console.log('AUTH URRLLLLLLM    is ', AUTH_URL);

		const authWindow = new WebviewWindow('google-oauth', {
			url: AUTH_URL,
			title: 'Google OAuth',
			width: 400,
			height: 400,
			resizable: true
		});
		console.log('Created the webview window');

		authWindow.once('tauri://created', function () {
			console.log("WebView window created");
		});

		authWindow.once('tauri://error', function (e) {
			console.error('Error creating WebView window:', e);
		});

		// Listen for the window to be closed
		authWindow.once('tauri://close-requested', function () {
			console.log("OAuth window was closed");
		});

		// ----------------------------------------------------------------
		// authWindow
		// 	.once('tauri://created', () => {
		// 		console.log('Inside this function11111111111111');
		// 		const intervalId = setInterval(() => {
		// 			// it is coming there each time and not getting the code because WebviewWindow is just loading the page itself and not AUTH_URL and hence not doing anything
		// 			const element = document.querySelector('textarea#code, input#code');
		// 			const code = (element as HTMLInputElement | HTMLTextAreaElement)?.value || '';

		// 			if (code) {
		// 				console.log('Gott the code as ', code);
		// 				clearInterval(intervalId);
		// 				getTokens(code)
		// 					.then(() => {
		// 						console.log('Closing the window now');
		// 						authWindow.close();
		// 						console.log('Closed the window');
		// 					})
		// 					.catch((error) => {
		// 						console.error('Error getting tokens:', error);
		// 					});
		// 			}
		// 		}, 500);
		// 	})
		// 	.catch((error) => {
		// 		console.error('Error setting up the window:', error);
		// 	});

		//------------------------------------------------------------------

		//window.location.href = AUTH_URL;
	}

	async function getTokens(code: string) {
		console.log('Inside this funciton');
		const tokenUrl = 'https://oauth2.googleapis.com/token';
		const response = await fetch(tokenUrl, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/x-www-form-urlencoded'
			},
			body: new URLSearchParams({
				client_id: CLIENT_ID,
				client_secret: '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',
				code: code,
				redirect_uri: REDIRECT_URI,
				grant_type: 'authorization_code'
			})
		});

		const data = await response.json();
		console.log('Token response:', data);
		// Store these tokens securely and use them for API requests
	}

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const code = params.get('code');
		if (code) {
			selectedSource = 'Google Drive';
		}
	});

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
		'Google Cloud Storage'
	];

	let showModal = false;
	let modalMessage = '';

	function selectSource(sourceName: string) {
		if (sourceName === 'Google Drive') {
			login().catch(console.error);
			selectedSource = 'Google Drive';
			setIsVisible();
		}
		if (premiumSources.includes(sourceName)) {
			modalMessage = 'This feature is available only in premium';
			showModal = true;
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
