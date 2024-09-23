<script type="ts">
	import { onMount } from 'svelte';
	import { googleDriveCode } from '../../stores/appState';

	onMount(() => {
		console.log('Reached here...');
		const queryParams = new URLSearchParams(window.location.search);
		let code = queryParams.get('code');

		console.log('COde    ', code);
		if (code) {
			console.log('Setting the code here');
			googleDriveCode.set(code);
			$googleDriveCode = code;
		}

		console.log('Value of the code here is ', $googleDriveCode);
	});
</script>

Please close the browser and open the app again
