<script>
	import '../../app.pcss';
	import Footer from './Footer.svelte';
	import Navbar from './Navbar.svelte';
	import Sidebar from './Sidebar.svelte';
	import { isLicenseVerified } from '../../stores/appState';

	let drawerHidden = false;
</script>

<div class:pointer-events-none={!$isLicenseVerified}>
	<header
		class="sticky top-0 z-40 mx-auto w-full flex-none border-b border-gray-200 bg-white dark:border-gray-600 dark:bg-gray-800"
	>
		<Navbar bind:drawerHidden />
	</header>
	<div class="overflow-hidden lg:flex">
		<Sidebar bind:drawerHidden />
		<div class="relative h-full w-full overflow-y-auto lg:ml-64">
			<slot />
			<Footer />
		</div>
	</div>
</div>
