<script>
  import S3Icon from "$lib/images/awsIcon.svg";
  import GCSIcon from "$lib/images/gcsIcon.svg";
  import AzureIcon from "$lib/images/azureIcon.svg";
  import OneDriveIcon from "$lib/images/oneDrive.png";
  import GoogleDriveIcon from "$lib/images/googleDriveIcon.svg";
  import localFileSystemIcon from "$lib/images/localFileSystemIcon.svg";
  import dropboxIcon from "$lib/images/dropboxIcon.svg";
  import webscraperIcon from "$lib/images/webscraperIcon.svg";
  import { tabs } from "./sidebarStore";
  import Counter from "./Counter.svelte";
  import GcsCollectorComponent from "./individualCollectorComponents/gcsCollectorComponent.svelte";
  import AzureCollectorComponent from "./individualCollectorComponents/azureCollectorComponent.svelte";
  import GdriveCollectorComponent from "./individualCollectorComponents/gdriveCollectorComponent.svelte";
  import AwsCollectorComponent from "./individualCollectorComponents/awsCollectorComponent.svelte";
  import DropboxCollectorComponent from "./individualCollectorComponents/dropboxCollectorComponent.svelte";
  import OnedriveCollectorComponent from "./individualCollectorComponents/onedriveCollectorComponent.svelte";
  import LocalfileCollectorComponent from "./individualCollectorComponents/localfileCollectorComponent.svelte";
  import WebscraperCollectorComponent from "./individualCollectorComponents/webscraperCollectorComponent.svelte";

  const collectors = [
    {
      id: "GCS",
      text: "GCS Collector",
      iconSrc: GCSIcon,
      component: GcsCollectorComponent,
    },
    {
      id: "Azure",
      text: "Azure Collector",
      iconSrc: AzureIcon,
      component: AzureCollectorComponent,
    },
    {
      id: "GoogleDrive",
      text: "Drive Collector",
      iconSrc: GoogleDriveIcon,
      component: GdriveCollectorComponent,
    },
    {
      id: "Webscraper",
      text: "Webscraper",
      iconSrc: webscraperIcon,
      component: WebscraperCollectorComponent,
    },
    {
      id: "S3",
      text: "AWS S3 Collector",
      iconSrc: S3Icon,
      component: AwsCollectorComponent,
    },
    {
      id: "Dropbox",
      text: "Dropbox Collector",
      iconSrc: dropboxIcon,
      component: DropboxCollectorComponent,
    },
    {
      id: "OneDrive",
      text: "One Drive Collector",
      iconSrc: OneDriveIcon,
      component: OnedriveCollectorComponent,
    },
    {
      id: "LocalFileSystem",
      text: "Local File Collector",
      iconSrc: localFileSystemIcon,
      component: LocalfileCollectorComponent,
    },
  ];

  /**
   * @param {{ id?: string; text: any; iconSrc?: string; }} collector
   */
  function onClickCollector(collector) {
    console.log("you clicked on: ", collector.text);

    // Check if a tab with the same title already exists
    const existingTab = $tabs.find((tab) => tab.title === collector.text);

    if (!existingTab) {
      // If the tab doesn't exist, create a new one
      const newTab = {
        title: collector.text,
        content: collector.component,
      };

      // Update the tabs array in the store
      tabs.update((currentTabs) => [...currentTabs, newTab]);
    }
  }
</script>

<div class="collectorListContainer">
  <div class="collectorListHeading">Collectors</div>
  <div class="supportText">
    To establish a connection with an external/internal data source where your
    files are stored.
  </div>

  {#each collectors as collector (collector.id)}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="collectorList" on:click={() => onClickCollector(collector)}>
      <div class="iconContainer">
        <img src={collector.iconSrc} alt="S" class="iconImage" />
      </div>
      <div class="collectorName">
        <p>{collector.text}</p>
      </div>
    </div>
  {/each}
</div>

<style>
  .collectorListContainer {
    /* margin-left: 3%; */
    margin-top: 1em;
    color: #2c3e50;
  }
  .supportText {
    font-size: 0.7em;
    margin-top: 5%;
    color: #5d6d7e;
    /* color: #2f2c41; */
  }

  .collectorListHeading {
  }

  .collectorList {
    margin-top: 10%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 0.5em;
    padding: 0 0.8em;
    border-radius: 0.5em;
    cursor: pointer;
  }
  .collectorList:hover {
    background-color: #82e0aa; /* Change the background color on hover */
    /* color: #1673c5; Change the text color on hover */
  }

  .iconContainer {
    width: 17%;
  }
  .iconImage {
    width: 100%;
    border-radius: 50%;
  }

  .collectorName {
    font-size: 0.8em;
  }
</style>
