<script>
export let data;
</script>

<style>
  nav {
    height: 3rem;
    padding: 0.25rem;
    display: flex;
    flex-direction: row;
    align-items: center;

    font-weight: bold;

    background-color: rgb(251, 250, 254);

    box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);

    position: sticky;
    top: 0;
  }

  .container {
    background-color: rgb(251, 250, 254);
    min-width: 100vw;
    min-height: calc(100vh - 3rem);
  }

  a {
    padding: 0.25rem;
    text-decoration: none;
    color: black;
  }

  .tournament_name {
    padding-right: 5px;
    border-right: 1px solid #ccc;
  }
</style>

<nav>
  <span class="tournament_name">{data.tournamentName}</span>
  {#each data.additionalLinks as link}
    <a href="{link.url}">{link.name}</a>
  {/each}
  <slot name="header" />
</nav>

<div class="container">
  <slot />
</div>
