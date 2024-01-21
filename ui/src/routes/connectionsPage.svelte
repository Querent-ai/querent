<script>
  import AddConnectionButton from "$lib/images/plusButton.svg";
  import TableComponent from "./common/tableComponent.svelte";
  import GCSIcon from "$lib/images/gcsIcon.svg";
  import AzureIcon from "$lib/images/azureIcon.svg";
  import OneDriveIcon from "$lib/images/oneDrive.svg";
  import GoogleDriveIcon from "$lib/images/googleDriveIcon.svg";
  import localFileSystemIcon from "$lib/images/localFileIcon.svg";
  import dropboxIcon from "$lib/images/dropboxIcon.svg";
  import S3Icon from "$lib/images/awsIcon.svg";

  import { onMount } from "svelte";

  let isDropdownOpen = false;

  const services = [
    {
      name: "Google Drive",
      icon: GoogleDriveIcon,
      authUrl: "http://localhost:3000/auth/googleDrive",
    },
    {
      name: "Google Cloud",
      icon: GCSIcon,
      authUrl: "http://localhost:3000/auth/googleCloud",
    },
    {
      name: "Dropbox",
      icon: dropboxIcon,
      authUrl: "http://localhost:3000/auth/dropbox",
    },
    {
      name: "One Drive",
      icon: OneDriveIcon,
      authUrl: "http://localhost:3000/auth/oneDrive",
    },
    {
      name: "Azure",
      icon: AzureIcon,
      authUrl: "http://localhost:3000/auth/azure",
    },
    {
      name: "AWS S3",
      icon: S3Icon,
      authUrl: "",
    },
    {
      name: "One Drive",
      icon: OneDriveIcon,
      authUrl: "",
    },
  ];

  let tableData = [];

  // Fetch table data when the component mounts
  onMount(async () => {
    try {
      const response = await fetch(
        "http://localhost:3000/collector-table-data"
      );
      if (response.ok) {
        tableData = await response.json();
      } else {
        console.error("Error fetching table data: ${response.status}");
      }
    } catch (error) {
      console.error("Error:", error.message);
    }
  });

  async function addServiceToTable(serviceData) {
    try {
      const response = await fetch(
        "http://localhost:3000/collector-table-data",
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(serviceData),
        }
      );

      if (response.ok) {
        // Update the local table data
        const newService = await response.json();
        tableData = [...tableData, newService.item];
      } else {
        console.error("Error adding service: ${response.status}");
      }
    } catch (error) {
      console.error("Error:", error.message);
    }
  }

  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
  }

  function handleServiceButtonClick(service) {
    const authWindow = window.open(
      service.authUrl,
      "authWindow",
      "width=500,height=600"
    );

    function onMessageReceived(event) {
      try {
        if (event.data === "authComplete") {
          console.log(`Authentication complete for ${service.name}`);
          const serviceData = {
            name: service.name,
            id: "1234567890", // This should be dynamic based on the service
            progress: 50, // This should be dynamic based on the service
          };

          addServiceToTable(serviceData);
        }
      } catch (error) {
        console.error(`Error during ${service.name} authentication:`, error);
        // Optionally, update the UI to reflect the error
      } finally {
        window.removeEventListener("message", onMessageReceived);
        if (authWindow) {
          authWindow.close();
        }
      }
    }

    window.addEventListener("message", onMessageReceived);
  }
</script>

<div class="connectionPageContainer">
  <div class="connectionHeader">
    <div class="connectionButton">
      Connections
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
      <img
        src={AddConnectionButton}
        alt="Button Icon"
        class="addConnectorIcon"
        on:click={toggleDropdown}
      />
    </div>
    {#if isDropdownOpen}
      <div class="dropdown">
        <p>Select a new connector to begin integration</p>
        <div class="dropdownMenu">
          {#each services as service}
            <!-- svelte-ignore a11y-click-events-have-key-events -->
            <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
            <p on:click={() => handleServiceButtonClick(service)}>
              <img src={service.icon} alt={`${service.name} Icon`} />
              {service.name}
            </p>
          {/each}
        </div>
      </div>
    {/if}
    <div class="descriptionText">
      Streamline your digital life with our intuitive platform, designed to
      connect various services - Drive, Cloud, and more - using OAuth for
      secure, seamless integration. Effortlessly manage and access all your
      accounts in one place, enhancing productivity and simplifying your online
      experience with robust, user-friendly security.
    </div>
  </div>
  <div class="connectionTable"><TableComponent data={tableData} /></div>
</div>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Manrope:wght@200;300;400;500;600;700;800&family=Montserrat:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap");

  .connectionPageContainer {
    height: inherit;
    display: flex;
    flex-direction: column;
    gap: 5em;
  }

  .connectionHeader {
    display: flex;
    gap: 20%;
  }
  .connectionButton {
    font-weight: 600;
    font-family: "Montserrat", sans-serif;
    color: black;
    font-size: 1.1em;

    display: flex;
    flex-direction: column;
    gap: 0.5em;
  }

  .addConnectorIcon {
    width: 30%;
    cursor: pointer;
  }

  .descriptionText {
    /* font-size: 0.5em; */
    font-family: "Manrope", sans-serif;
    font-weight: 300;
    font-size: 0.9em;
    width: 60%;
  }

  .dropdown {
    position: absolute;
    top: 15%;
    background-color: white;
    width: 170px;
    box-shadow: 0px 8px 16px 0px rgba(0, 0, 0, 0.2);
    z-index: 1;
    border-radius: 5px;
  }

  .dropdown p {
    font-size: 0.7em;
    padding: 10px;
  }

  .dropdownMenu {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .dropdownMenu p {
    display: flex;
    gap: 0.6em;
    font-family: "Montserrat", sans-serif;

    font-weight: 200;
    font-size: 0.9em;
    margin: 0;

    padding: 8%;
  }

  .dropdownMenu p:hover {
    background-color: #e2e8f0;
    cursor: pointer;
  }

  .dropdownMenu p img {
    width: 15%;
  }
</style>
