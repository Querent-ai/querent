<script>
  // import docs from "$lib/images/docs.png";
  // import github from "$lib/images/github.png";
  // import discord from "$lib/images/discord.png";
  import github from "$lib/images/githubIcon.svg";
  import docs from "$lib/images/docsIcon.svg";
  import discord from "$lib/images/discordIcon.svg";
  import InPlaceEdit from "./InPlaceEdit.svelte";

  let fileName = "Untitled";

  let showDropdown = false;

  function toggleDropdown() {
    showDropdown = !showDropdown;
  }

  /**
   * @param {string} field
   */
  function submit(field) {
    // @ts-ignore
    return ({ detail: newValue }) => {
      // IRL: POST value to server here
      console.log(`updated ${field}, new value is: "${newValue}"`);
    };
  }

  function handleButtonClick() {
    const authWindow = window.open(
      "http://localhost:3000/auth/googleDrive",
      "authWindow",
      "width=500,height=600"
    );

    // This function is defined inside to have access to authWindow.
    /**
     * @param {{ data: string; }} event
     */
    function onMessageReceived(event) {
      if (event.data === "authComplete") {
        window.removeEventListener("message", onMessageReceived); // Cleanup the listener
      }
    }

    window.addEventListener("message", onMessageReceived);
  }

  function handleDropboxButtonClick() {
    const authWindow = window.open(
      "http://localhost:3000/auth/dropbox",
      "authWindow",
      "width=500,height=600"
    );
  }

  function handleGCloudButtonClick() {
    const authWindow = window.open(
      "http://localhost:3000/auth/googleCloud",
      "authWindow",
      "width=500,height=600"
    );
  }

  function handleOneDriveButtonClick() {
    const authWindow = window.open(
      "http://localhost:3000/auth/oneDrive",
      "authWindow",
      "width=500,height=600"
    );
  }
</script>

<header>
  <div class="headerOptions">
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <p on:click={toggleDropdown}>Support</p>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
    <p on:click={handleButtonClick}>GoogleSignin</p>
    <p on:click={handleDropboxButtonClick}>DropboxSignin</p>
    <p on:click={handleGCloudButtonClick}>GCloudSignin</p>
    <p on:click={handleOneDriveButtonClick}>OneDriveSignin</p>

    <p>Account</p>
    {#if showDropdown}
      <div class="dropdownMenu">
        <p>Our Docs</p>
        <p>Github</p>
        <p>Discord</p>
      </div>
    {/if}
  </div>
</header>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Montserrat:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap");

  header {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    width: 100%;
    height: 9vh;
    /* background-color: #1b192e; */
    color: #2c3e50;
  }

  .headerOptions {
    font-family: "Montserrat", sans-serif;
    display: flex;
    gap: 5em;
    padding-right: 3em;
    cursor: pointer;
    position: relative;
  }

  .dropdownMenu {
    position: absolute;
    top: 100%; /* Position right below the Support button */
    left: 0; /* Align to the right edge of .headerOptions */
    background-color: #fff; /* or any color you prefer */
    border: 1px solid #ddd; /* Just some styling */
    border-radius: 4px;
    width: 150px; /* Adjust as necessary */
    box-shadow: 0px 8px 16px 0px rgba(0, 0, 0, 0.2);
    z-index: 1;
  }

  .dropdownMenu p {
    padding: 12px 16px;
    margin: 0;
    text-align: left;
    white-space: nowrap;
  }
  .dropdownMenu p:hover {
    background-color: #f1f1f1; /* Change as desired for hover effect */
  }
</style>
