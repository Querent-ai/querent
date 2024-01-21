<script>
  export let data = [];
</script>

<table class="table">
  <thead>
    <tr>
      <th>Instance Name</th>
      <th>ID</th>
      <th>Progress</th>
    </tr>
  </thead>
  <tbody>
    {#each data as row}
      <tr>
        <td class="rowName">{row.name}</td>
        <td>{row.id}</td>
        <td class="progressRow">
          <span class="progressLabel">
            {row.progress}%
          </span>
          <div class="progress-container">
            <div class="progress-bar" style="width: {row.progress}%;">
              <!-- The progress bar itself -->
            </div>
          </div>
        </td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  @import url("https://fonts.googleapis.com/css2?family=Manrope:wght@200;300;400;500;600;700;800&family=Montserrat:ital,wght@0,100;0,200;0,300;0,400;0,500;0,600;0,700;0,800;0,900;1,100;1,200;1,300;1,400;1,500;1,600;1,700;1,800;1,900&display=swap");

  .table {
    border-collapse: collapse;
    width: 95%;
    border: none;
    font-family: "Montserrat", sans-serif;
    color: black;
  }

  tr {
    border-bottom: 1px solid #ccc;
  }

  tr:last-child {
    border-bottom: none;
  }

  th,
  td {
    text-align: left;
    padding: 8px;
    width: 30%;
  }

  .rowName {
    font-weight: 600;
  }

  .progressRow {
    text-align: right;
    justify-content: center;
  }
  .progress-container {
    width: 100%;
    background-color: #e0e0e0;
    border-radius: 8px;
  }

  .progress-bar {
    height: 7px;
    background-color: #4caf50;
    border-radius: 8px;
    transition: width 0.5s ease;
  }

  .progressLabel {
    font-size: 0.6em;
  }
</style>
