<script>
  import TournamentCard from './TournamentCard.svelte';

    import Card from "$lib/Card.svelte";
    import CardButton from "$lib/CardButton.svelte";
    import { makeRequest } from '$lib/api';
    import { onMount } from 'svelte';

    export let data;

    let tournamentsInProgress = data.tournamentsInfo.active;

    let tournamentsUpcoming = data.tournamentsInfo.upcoming;
</script>


<style>
    .list {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;
        flex-wrap: wrap;

        width: 100%;
    }
    
    .card {
        padding: 10px;
        border: 1px solid #aaa;
        font-weight: bold;
        align-items: center;
        line-height: 1rem;
        border-radius: 5px;
        margin-right: 5px;
        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);
        background-color: white;
    }

    h2 {
        font-weight: bold;
    }

    h3 {
        font-weight: bold;
    }

    .container {
        max-width: 750px;
        margin: 0 auto;
        background-color: rgb(251, 250, 254);
    }

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

    .back {
        background-color: rgb(251, 250, 254);
        width: 100vw;
        height: 100vh;
    }

    .img-container {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;
    }

    .img-container img {
        margin-top: 8px;
        height: 50px;
    }
    
    .card-content {
        width: 15rem;
    }

    .card-content h3 {
        margin: 5px;
    }

    .card-content img {
        padding-bottom: 5px;
    }

    .note {
        font-style: italic;
    }

    .section {
        margin-bottom: 10px;
    }
</style>

<div class="container">
    <div class="section">
    <h2>In Progress</h2>
    {#if tournamentsInProgress.length === 0}
        <p class="note">There are no tournaments in progress right now.</p>
    {:else}
    <div class="list">
        {#each tournamentsInProgress as tournament}
            <TournamentCard tournament={tournament} />
        {/each}
    </div>
    {/if}
    </div>
    
    <div class="section">
    <h2>Upcoming</h2>
    {#if tournamentsUpcoming.length === 0}
        <p class="note">There are no upcoming tournaments right now.</p>
    {:else}
        <div class="list">
        {#each tournamentsUpcoming as tournament}
            <TournamentCard tournament={tournament} />
        {/each}
        </div>
    {/if}
    </div>
</div>
