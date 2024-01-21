<script>
  import tabIcon from "$lib/images/tabIcon.svg";
  import closeIcon from "$lib/images/closeIcon.svg";
  import { tabs } from "./sidebarStore";

  let activeTab = 0;

  /**
   * @param {number} tabIndex
   */
  function switchTab(tabIndex) {
    activeTab = tabIndex;
  }

  /**
   * @param {number} tabIndex
   */
  function closeTab(tabIndex) {
    $tabs.splice(tabIndex, 1);
    if (activeTab >= $tabs.length) {
      activeTab = $tabs.length - 1;
    }
  }
</script>

<div class="tabbed-interface">
  <div class="tabButtonContainer">
    {#each $tabs as tab, index (tab.title)}
      <div class="individualTab">
        <button
          class="tabButton {activeTab === index ? 'active' : ''}"
          on:click={() => switchTab(index)}
        >
          <img src={tabIcon} class="tabIconImage" alt="img" />
          <span class="tabTitleText">{tab.title}</span>
          <button class="closeButton" on:click={() => closeTab(index)}>
            <!-- <span>x</span> -->
            <img src={closeIcon} alt="close" />
          </button>
        </button>
        <div class="divider" />
      </div>
    {/each}
  </div>
  <div class="tab-content">
    {#if $tabs[activeTab]}
      <div class="tab-content-item">
        <svelte:component this={$tabs[activeTab].content} />
      </div>
    {/if}
  </div>
</div>

<style>
  /* Add your tabbed interface styles here */
  .tabbed-interface {
  }

  .tabButtonContainer {
    /* background-color: #1b192e; */
    background-color: #f7f7f7;

    display: flex;
    align-items: center;
    width: 100%;
    align-items: center;
  }

  .individualTab {
    width: 15%;
    display: flex;
    align-items: center;
    /* color: white; */
  }

  .divider {
    border: 0.5px solid #5d6d7e;
    height: 15px;
  }

  .tabButton {
    display: flex;
    align-items: center;
    justify-content: space-evenly;
    gap: 0.2em;
    font-size: 0.8em;
    height: 36px;
    width: 100%;

    border-top-left-radius: 0.5em;
    border-top-right-radius: 0.5em;
    /* color: white; */
    border: none;
    cursor: pointer;
    background-color: #f7f7f7;
  }

  .tabButton:hover {
    background-color: white;
  }

  .tabTitleText {
    white-space: nowrap;
    overflow: scroll;
    text-overflow: ellipsis;
  }

  .tabIconImage {
    width: 12%;
  }

  .tabButtonContainer button.active {
    background-color: white;
    /* box-shadow: 2px 2px 5px rgba(192, 192, 192, 0.5); */
    box-shadow: 0px -1px #888888;
    border: none;
  }

  .closeButton {
    background-color: transparent;
    border: none;
    cursor: pointer;
    margin-left: 8px;
    color: #2c3e50;

    width: 20%;
    display: flex;
  }

  .closeButton img {
    width: 100%;
  }

  .tab-content {
    /* background-color: #2f2c41; */
    width: 100%;
    height: 87vh;
    padding-top: 1%;
    /* border: 1em solid red; */
    border-radius: 1em;
  }

  .tab-content-item {
    /* Add your styles for tab content */
  }
</style>
