<script>
  import GoogleDriveIcon from "$lib/images/googleDriveIcon.svg";
  import AuthenticationCompleteModal from "../common/AuthenticationCompleteModal.svelte";
  import AuthenticationCompleteGoogleDrive from "./CollectorPopupSlots/AuthenticationCompleteGoogleDrive.svelte";
  let showLoadModal = false;

  function handleButtonClick() {
    const authWindow = window.open(
      "http://localhost:3000/auth/google",
      "authWindow",
      "width=500,height=600"
    );

    // This function is defined inside to have access to authWindow.
    /**
     * @param {{ data: string; }} event
     */
    function onMessageReceived(event) {
      if (event.data === "authComplete") {
        showLoadModal = true;
        window.removeEventListener("message", onMessageReceived); // Cleanup the listener
      }
    }

    window.addEventListener("message", onMessageReceived);
  }

  function closeModal() {
    showLoadModal = false;
  }
</script>

<div class="collectorContentContainer">
  <div class="collectorDetailedHeader">
    <div class="collectorIconContainer">
      <img src={GoogleDriveIcon} alt="GCS_Icon" class="collectorIcon" />
    </div>
    <div class="collectorInfo">
      <p class="collectorTitle">Google Drive Collector</p>
      <p class="collectorDescription">
        Collector tool designed for real-time data streaming from Google Drive,
        enabling seamless and efficient extraction of data.
      </p>
      <div class="buttonContainer">
        <button class="createCollectorButton" on:click={handleButtonClick}
          >Create Instance</button
        >
        {#if showLoadModal}
          <AuthenticationCompleteModal
            bind:isOpen={showLoadModal}
            close={closeModal}
          >
            <!-- <button on:click={loadFolders}> Load your folders </button> -->
            <AuthenticationCompleteGoogleDrive />
          </AuthenticationCompleteModal>
        {/if}
      </div>
    </div>
  </div>
  <div class="collectorInstanceTable" />
</div>

<style>
  @import "collectorStyle.css";
</style>
