<script>
  import ConnectionsPage from "./connectionsPage.svelte";
  import Sidebar from "./Sidebar.svelte";
  import colorStore from "./colorStore";
  import "./styles.css";
  import { showCollectorComponent } from "./sidebarStore";

  // let colors;

  // Subscribe to the color store
  // colorStore.subscribe((value) => {
  //   colors = value;
  // });
</script>

<div class="app">
  <div class="appContainer">
    <div class="sidebarComponent">
      <Sidebar />
    </div>
    <div class="workspaceContainer">
      {#if $showCollectorComponent}
        <ConnectionsPage />
      {/if}
    </div>
  </div>
</div>

<style>
  .appContainer {
    display: flex;
    background-color: #f8f9fa;
    height: 100vh;
  }

  .workspaceContainer {
    width: 100%;
    padding-top: 2.5%;
    padding-left: 2.5%;
    background-color: white;
  }
</style>
