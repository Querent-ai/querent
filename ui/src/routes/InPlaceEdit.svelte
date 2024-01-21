<script>
  import { createEventDispatcher, onMount } from "svelte";

  /**
   * @type {any}
   */
  export let value,
    required = true;

  const dispatch = createEventDispatcher();
  /**
   * @type {any}
   */
  let original,
    editing = false;

  onMount(() => {
    original = value;
  });

  function edit() {
    editing = true;
  }

  function submit() {
    if (value != original) {
      if (value == null) {
        value = original;
      editing = false;
      }
      dispatch("submit", value);
    }

    editing = false;
  }

  /**
   * @param {{ key: string; preventDefault: () => void; }} event
   */
  function keydown(event) {
    if (event.key == "Escape") {
      event.preventDefault();
      value = original;
      editing = false;
    }
  }

  /**
   * @param {HTMLInputElement} element
   */
  function focus(element) {
    element.focus();
  }
</script>

{#if editing}
  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <form on:submit|preventDefault={submit} on:keydown={keydown}>
    <input bind:value on:blur={submit} {required} use:focus />
  </form>
{:else}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div on:click={edit}>
    {value}
  </div>
{/if}

<style>
  input {
    border: none;
    background: none;
    font-size: inherit;
    color: inherit;
    font-weight: inherit;
    text-align: inherit;
    box-shadow: none;
  }
</style>
