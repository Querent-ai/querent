<script>
	import { createEventDispatcher } from 'svelte';
	import { commands } from '../../service/bindings-jsdoc';

	const dispatch = createEventDispatcher();
	let email = '';
	let key = '';
	let verificationMessage = '';

	async function verifyLicense() {
		try {
			if (email && key) {
				let res = await commands.setRianLicenseKey(key);
				if (res) {
					console.log('Valid key');
					dispatch('close', { verified: true });
				} else {
					console.log('Invalid key');
					verificationMessage = 'Failed to set the key. Please check your key and try again.';
				}
			} else {
				verificationMessage = 'Please fill in all fields.';
			}
		} catch (error) {
			console.error('Error verifying license:', error);
			verificationMessage = `An error occurred: ${error.message || 'Unknown error'}. Please try again.`;
		}
	}
</script>

<div class="modal-overlay">
	<div class="modal-content">
		<div class="description">
			<p class="mb-3 text-center text-xl font-semibold">License key not found</p>
			<p class="mb-5">Please enter your email and license key to activate your account.</p>
		</div>
		<form on:submit|preventDefault={verifyLicense}>
			<label for="Email" class="text-l font-semibold">Please enter your Email address</label>
			<input type="email" bind:value={email} placeholder="Email" required />
			<label for="License-key" class="text-l font-semibold">Please enter your License key</label>
			<input type="text" bind:value={key} placeholder="License Key" required />
			<button type="submit" class="mt-5">Submit</button>
		</form>
		<p class="message text-s">{verificationMessage}</p>

		<div class="divider"></div>
		<p class="info">OR</p>
		<p class="info">
			Get your license key at <a
				href="https://querent.xyz/rian"
				target="_blank"
				rel="noopener noreferrer">querent.xyz</a
			>
		</p>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		justify-content: center;
		align-items: center;
	}
	.modal-content {
		background: white;
		padding: 25px;
		overflow: auto;
		border-radius: 10px;
		width: 90%;
		max-width: 550px;
	}
	input[type='email'],
	input[type='text'] {
		width: 100%;
		padding: 8px;
		margin: 5px 0;
		border-radius: 5px;
	}
	button {
		display: block;
		width: 100%;
		padding: 10px;
		background-color: #007bff;
		color: white;
		border: none;
		border-radius: 5px;
		cursor: pointer;
	}
	button:hover {
		background-color: #0056b3;
	}
	.info {
		margin-top: 20px;
		text-align: center;
	}
	a {
		color: #007bff;
	}

	.divider {
		height: 2px;
		background: #ccc;
		margin: 10px 0;
	}
</style>
