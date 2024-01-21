<script>
  import localFileSystemIcon from "$lib/images/localFileSystemIcon.svg";
</script>

<div class="collectorContentContainer">
  <div class="collectorDetailedHeader">
    <div class="collectorIconContainer">
      <img src={localFileSystemIcon} alt="GCS_Icon" class="collectorIcon" />
    </div>
    <div class="collectorInfo">
      <p class="collectorTitle">Local File System Collector</p>
      <p class="collectorDescription">
        Collector tool designed for real-time data streaming from your local
        file system, enabling seamless and efficient extraction of data.
      </p>
      <div class="buttonContainer">
        <button class="createCollectorButton">Create Instance</button>
      </div>
    </div>
  </div>
  <div class="collectorInstanceTable" />
</div>

<style>
  @import "collectorStyle.css";
</style>
