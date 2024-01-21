<!-- AuthenticationCompleteModal.svelte -->
<script>
  export let isOpen = false;

  // Close function
  export let close = () => {
    isOpen = false;
  };

  let modal; // Declare the modal variable

  // Reactive statement to handle modal display style
  $: modalDisplay = isOpen ? "block" : "none";
</script>

<div class="modal" bind:this={modal} style="display: {modalDisplay}">
  <div class="modal-content">
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <!-- <span class="close" on:click={close}>&times;</span> -->
    <slot />
  </div>
</div>

<style>
  .modal {
    position: fixed;
    z-index: 1;
    left: 0;
    top: 0;
    width: 100%;
    height: 100%;
    overflow: auto;
    background-color: rgba(0, 0, 0, 0.4);
  }

  .modal-content {
    background-color: #fefefe;
    margin: 15% auto;
    padding: 10px;
    border: 1px solid #888;
    width: 53%;
  }

  .close {
    color: #aaaaaa;
    float: right;
    font-size: 28px;
    font-weight: bold;
  }
  .close:hover,
  .close:focus {
    color: #000;
    text-decoration: none;
    cursor: pointer;
  }
</style>
