<script>
  import "../app.css";
  export let data;
</script>
  
  <style>
    nav {
      min-height: 3rem;
      padding: 0.25rem;
      display: flex;
      flex-direction: row;
      align-items: center;
      flex-wrap: wrap;
  
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

      padding: 1rem;
    }

    .container.full {
      min-height: 100vh;
      padding: 0;
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
    
    .login {
      margin-left: auto;
    }

    div.login {
      display: flex;
      align-items: center;
    }
    div.login > a {
      border-right: 1px solid #ccc;
      padding-right: 2px;
      margin-right: 2px;
    }
  </style>
  
  {#if !data.hideNavbar}
  <nav>
    <a class="tournament_name" href={data.titleLink}>{data.pageTitle}</a>
    {#each data.additionalLinks as link}
      <a href="{link.url}">{link.name}</a>
    {/each}
    <slot name="header" />
    {#if data["isAuthenticated"]}
    <div class="login">
    <a href={`/user/${data.userId}`}>Profile</a>
    <form method="POST" action="/logout">
      <button type="submit" formaction="/logout">Logout</button>
    </form>
    </div>
    {:else}
      <a class="login" href="/login">Login</a>
    {/if}
  </nav>
  {/if}
  
  <div class="{data.hideNavbar ? 'container full' : 'container'}">
    <slot />
  </div>
