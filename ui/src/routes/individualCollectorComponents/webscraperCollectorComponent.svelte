<script>
  import webscraperIcon from "$lib/images/webscraperIcon.svg";
</script>

<div class="collectorContentContainer">
  <div class="collectorDetailedHeader">
    <div class="collectorIconContainer">
      <img src={webscraperIcon} alt="GCS_Icon" class="collectorIcon" />
    </div>
    <div class="collectorInfo">
      <p class="collectorTitle">Webscraper</p>
      <p class="collectorDescription">
        Collector tool designed for real-time data streaming from any website
        link you provide, enabling seamless and efficient extraction of data.
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
