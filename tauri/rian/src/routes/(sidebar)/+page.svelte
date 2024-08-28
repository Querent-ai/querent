<script lang="ts">
	import type { PageData } from './$types';
	import Dashboard from '../../lib/dashboard/Dashboard.svelte';
	import MetaTag from '../utils/MetaTag.svelte';
	import { commands } from '../../service/bindings';
	import LicenseKeyModal from './LicenseKeyModal.svelte';
	import { onMount } from 'svelte';
	import { isLicenseVerified } from '../../stores/appState';
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
			console.error('Error checking license key:', error);
			alert(`Failed to check license key: ${error.message || error}`);
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
<main class="p-4">
	<Dashboard {data} />
</main>
