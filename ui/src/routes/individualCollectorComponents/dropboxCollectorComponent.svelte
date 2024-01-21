<script>
  import dropboxIcon from "$lib/images/dropboxIcon.svg";
</script>

<div class="collectorContentContainer">
  <div class="collectorDetailedHeader">
    <div class="collectorIconContainer">
      <img src={dropboxIcon} alt="GCS_Icon" class="collectorIcon" />
    </div>
    <div class="collectorInfo">
      <p class="collectorTitle">Dropbox Collector</p>
      <p class="collectorDescription">
        Collector tool designed for real-time data streaming from Dropbox,
        enabling seamless and efficient extraction of data.
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
