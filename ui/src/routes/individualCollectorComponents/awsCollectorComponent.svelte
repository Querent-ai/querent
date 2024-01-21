<script>
  import S3Icon from "$lib/images/awsIcon.svg";
</script>

<div class="collectorContentContainer">
  <div class="collectorDetailedHeader">
    <div class="collectorIconContainer">
      <img src={S3Icon} alt="GCS_Icon" class="collectorIcon" />
    </div>
    <div class="collectorInfo">
      <p class="collectorTitle">Google Drive Collector</p>
      <p class="collectorDescription">
        Collector tool designed for real-time data streaming from AWS S3 Bucket,
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
