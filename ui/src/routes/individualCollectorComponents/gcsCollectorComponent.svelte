<script>
  import GCSIcon from "$lib/images/gcsIcon.svg";
  import TableComponent from "../common/tableComponent.svelte";
</script>

<div class="collectorContentContainer">
  <div class="collectorDetailedHeader">
    <div class="collectorIconContainer">
      <img src={GCSIcon} alt="GCS_Icon" class="collectorIcon" />
    </div>
    <div class="collectorInfo">
      <p class="collectorTitle">Google Cloud Collector</p>
      <p class="collectorDescription">
        Collector tool designed for real-time data streaming from Google Cloud
        Services, enabling seamless and efficient extraction of data.
      </p>
      <div class="buttonContainer">
        <button class="createCollectorButton">Create Instance</button>
      </div>
    </div>
  </div>
  <div class="collectorInstanceTable" />
  <TableComponent />
</div>

<style>
  @import "collectorStyle.css";
</style>
