<script>
	export let message = 'Loading...';
</script>

<div class="modal">
	<div class="modal-content">
		<p>{message}</p>
	</div>
</div>

<style>
	.modal {
		position: fixed;
		z-index: 1000;
		left: 0;
		top: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		justify-content: center;
		align-items: center;
	}

	.modal-content {
		background-color: white;
		padding: 20px;
		border-radius: 5px;
		text-align: center;
	}
</style>
