<script>
  import successCorrectIcon from "$lib/images/successCorrectIcon.svg";

  /**
   * @type {any[]}
   */
  let folders = [];
  let showModal = false;

  function loadFolders() {
    fetch("http://localhost:3000/drive/folders")
      .then((response) => response.json())
      .then((data) => {
        folders = data;
        showModal = true;
      });
  }

  function handleSubmit() {
    // You can add functionality to handle the submit action here
  }
</script>

<div class="container">
  <img
    src={successCorrectIcon}
    alt="AuthenticationCheck"
    class="authenticationIcon"
  />
  <h1>Authentication Complete</h1>
  <button on:click={loadFolders}>Load your Google Drive Folders</button>
</div>

<!-- Folders Load Modal -->
{#if showModal}
  <div class="modal">
    <div class="modal-header">
      <h2>Authentication Complete</h2>
    </div>
    <div class="modal-body">
      <p>Please choose the folders to use.</p>
      <ul>
        {#each folders as folder}
          <li>
            <input type="checkbox" checked={true} />
            {folder}
          </li>
        {/each}
      </ul>
    </div>
    <div class="modal-footer">
      <button on:click={handleSubmit} class="submit-btn">Submit</button>
    </div>
  </div>
{/if}

<style>
  .container {
    font-family: "Arial", sans-serif;
    height: 30vh;
    width: 100vh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    background-color: rgba(130, 224, 170, 0.5);
    padding: 15px;
  }

  .authenticationIcon {
    width: 10%; /* percentage-based width */
    max-width: 200px; /* max width to ensure it doesn't get too large */
    align-self: center; /* to ensure it's centered in flex container */
  }

  h1 {
    margin-bottom: 20px;
    color: #1967d2; /* Google blue */
  }

  button {
    padding: 10px 20px;
    font-size: 16px;
    background-color: #1a73e8; /* Google blue */
    color: #ffffff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
  }

  button:hover {
    background-color: #0f62cd; /* Slightly darker Google blue for hover */
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    height: 45vh;
    width: 120vh;
    background-color: white;
    display: flex;
    flex-direction: column;
    color: black;
    border-radius: 10px;
  }

  .modal-header {
    background: #1967d2; /* Google blue */
    padding: 5px;
    color: #fff;
    border-radius: 10px 10px 0 0;
    text-align: center;
  }

  .modal-body {
    flex-grow: 1;
    padding: 10px 20px;
    overflow-y: auto;
  }

  ul {
    list-style-type: none;
    padding: 0;
    margin: 0;
  }

  li {
    display: flex;
    align-items: center;
    margin: 10px 0;
  }

  input[type="checkbox"] {
    margin-right: 10px;
    padding: 10px;
  }

  .modal-footer {
    padding: 10px;
    display: flex;
    justify-content: flex-end;
  }

  .submit-btn {
    padding: 10px 20px;
    background-color: #4caf50; /* Green */
    color: #fff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
  }

  .submit-btn:hover {
    background-color: #45a049; /* Darker green for hover */
  }
</style>
