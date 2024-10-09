<script lang="ts">
	import type { PageData } from './$types';
	import Dashboard from '../../lib/dashboard/Dashboard.svelte';
	import MetaTag from '../utils/MetaTag.svelte';
	import { commands } from '../../service/bindings';
	import LicenseKeyModal from './LicenseKeyModal.svelte';
	import { onMount } from 'svelte';
	import { isLicenseVerified } from '../../stores/appState';
	import { Window } from '@tauri-apps/api/window';
	import { Webview } from '@tauri-apps/api/webview';
	import ErrorModal from '$lib/dashboard/ErrorModal.svelte';
	let showErrorModal = false;
	let errorMessage = '';
	function closeErrorModal() {
		showErrorModal = false;
	}
	export let data: PageData;

	const path: string = '';
	const description: string = 'Admin Dashboard example using Flowbite Svelte';
	const title: string = 'Querent Admin Dashboard - Home';
	const subtitle: string = 'Admin Dashboard';

	let res: boolean;

	$: hasKey = res;

	onMount(async () => {
		try {
			res = await commands.hasRianLicenseKey();
			isLicenseVerified.set(res);
		} catch (error) {
			const message = encodeURIComponent((error as any).toString());
			const urlWithMessage = `http://localhost:5173/error?error=${message}`;

			const appWindow = new Window('error-window', {
				height: 250,
				width: 500
			});

			const webview = new Webview(appWindow, 'error-window', {
				url: urlWithMessage,
				x: 0,
				y: 0,
				height: 100,
				width: 100
			});
		}
	});

	function handleModalClose(event: { detail: { verified: boolean } }) {
		if (event.detail.verified) {
			res = true;
			isLicenseVerified.set(true);
		}
	}
</script>

<MetaTag {path} {description} {title} {subtitle} />
{#if !hasKey}
	<div class="fixed inset-0 z-50 bg-black bg-opacity-50" style="pointer-events: none;"></div>
	<div class="fixed inset-0 z-[51] flex items-center justify-center" style="pointer-events: none;">
		<div style="pointer-events: auto;">
			<LicenseKeyModal on:close={handleModalClose} />
		</div>
	</div>
{/if}

{#if showErrorModal}
	<ErrorModal {errorMessage} closeModal={closeErrorModal} />
{/if}
<main class="p-4">
	<Dashboard {data} />
</main>
