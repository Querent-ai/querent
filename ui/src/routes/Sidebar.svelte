<script>
  import { writable } from "svelte/store";
  import logo from "$lib/images/querentMainLogo.svg";
  import leftArrows from "$lib/images/leftArrows.svg";
  import rightArrows from "$lib/images/rightArrows.svg";
  import connections from "$lib/images/connections.svg";
  import insights from "$lib/images/insightsIcon.svg";
  import storage from "$lib/images/storageIcon.svg";
  import qui from "$lib/images/quiAssistIcon.svg";
  import llm from "$lib/images/llmIcon.svg";
  import accountIcon from "$lib/images/accountIcon.svg";
  import settingsIcon from "$lib/images/settingsIcon.svg";
  import { showCollectorComponent } from "./sidebarStore";

  const isSidebarOpen = writable(false);

  const buttons = [
    { name: "Connections", icon: connections },
    { name: "LLM", icon: llm },
    { name: "QuiAssistant", icon: qui },
    { name: "Insights", icon: insights },
    { name: "Storage", icon: storage },
  ];

  /**
   * @type {string | null}
   */
  let activeButton = null; // Track the active button

  /**
   * @param {string | null} button
   */
  function onClickButton(button) {
    if (activeButton === button) {
      activeButton = null; // Deselect if clicking the active button

      showCollectorComponent.update((value) => !value); // Toggle the value // work on this
    } else {
      activeButton = button;
      const selectedButton = buttons.find((b) => b.name === button);
      if (selectedButton) {
        showCollectorComponent.update(
          (value) => selectedButton.name === "Connections"
        );
      }
    }
  }

  function toggleSidebar() {
    isSidebarOpen.update((state) => !state);
  }
</script>

<div class="sidebarContainer" class:expanded={$isSidebarOpen}>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <!-- svelte-ignore a11y-missing-attribute -->
  <a
    class="sidebarToogleIcon"
    class:expanded={$isSidebarOpen}
    on:click={toggleSidebar}
  >
    <img src={$isSidebarOpen ? logo : ""} class="mainQuerentLogo" />
    <img
      src={$isSidebarOpen ? rightArrows : leftArrows}
      alt="Toggle Sidebar"
      class="arrowsIcon"
    />
  </a>
  {#if $isSidebarOpen}
    <div class="sidebarComponents">
      {#each buttons as button}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="individualButton"
          class:active={activeButton === button.name}
          on:click={() => onClickButton(button.name)}
        >
          <img src={button.icon} alt="Button Icon" class="sidebarIcon" />
          <span class="serviceName">{button.name}</span>
        </div>
      {/each}

      <div class="accountSettings">
        <div class="accountSettingButton">
          <img src={accountIcon} alt="icon" />
          <span class="serviceName">Account</span>
        </div>
        <div class="accountSettingButton">
          <img src={settingsIcon} alt="icon" />
          <span class="serviceName">Settings</span>
        </div>
      </div>
    </div>
  {:else}
    <div class="sidebarComponents">
      {#each buttons as button}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="individualButton"
          class:active={activeButton === button.name}
          on:click={() => onClickButton(button.name)}
        >
          <img src={button.icon} alt="Button Icon" class="icon" />
        </div>
      {/each}
      <div class="accountSettings">
        <img src={accountIcon} alt="icon" class="accountSettingButton" />
        <img src={settingsIcon} alt="icon" class="accountSettingButton" />
      </div>
    </div>
  {/if}
</div>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap");

  .sidebarContainer {
    width: 50px;
    /* height: 100vh; */
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-right: 1em;
    padding-left: 1em;
    padding-top: 2em;
    color: black;
    transition: width 0.5s ease;
  }

  .sidebarContainer.expanded {
    width: 175px;
  }
  .sidebarToogleIcon.expanded {
    gap: 0.5em;
  }

  .sidebarToogleIcon {
    /* width: 50%; */
    display: flex;
    align-items: center;
  }

  .mainQuerentLogo {
    width: 87%;
  }

  .sidebarComponents {
    display: flex;
    flex-direction: column;
    align-items: baseline;
    padding-top: 3em;
    gap: 1em;
  }

  /* .sidebarButtons {
    display: flex;
    flex-direction: column;
    width: 100%;
    gap: 0.3em;
    margin-top: 1em;
  } */

  .individualButton {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1em;
    gap: 0.7em;
    cursor: pointer;
    border-radius: 1em;
    transition: background-color 0.3s ease;
  }

  .individualButton:hover,
  .individualButton.active {
    background-color: #e2e8f0;
  }

  .accountSettings {
    display: flex;
    flex-direction: column;
    gap: 2em;
    justify-content: center;
    align-items: center;
    padding: 1em;
    border-radius: 1em;
    margin-top: 8em;
    transition: background-color 0.3s ease;
  }

  .accountSettingButton {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.7em;
    cursor: pointer;
    border-radius: 1em;
  }

  .accountSettingButton:hover {
    background-color: #e2e8f0;
  }

  .serviceName {
    font-family: "Montserrat", sans-serif;
  }
</style>
