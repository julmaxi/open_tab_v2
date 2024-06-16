<script>
  import TournamentCard from './TournamentCard.svelte';

    import Card from "$lib/Card.svelte";
    import CardButton from "$lib/CardButton.svelte";

    let tournamentsInProgress = [
        {
            "name": "Tournament 1",
            "start": "2021-10-01T00:00:00Z",
            "end": "2021-10-02T00:00:00Z"
        },
        {
            "name": "Tournament 2",
            "start": "2021-10-01T00:00:00Z",
            "end": "2021-10-02T00:00:00Z"
        }
    ]

    let tournamentsUpcoming = [
        {
            "name": "Tournament 2",
            "start": "2021-10-03T00:00:00Z",
            "end": "2021-10-04T00:00:00Z"
        },
        {
            "name": "Tournament 3",
            "start": "2021-10-05T00:00:00Z",
            "end": "2021-10-06T00:00:00Z"
        }
    ]
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
</style>

<nav>
    <span>Open Tab</span>
</nav>

<div class="back">
<div class="container">
    {#if tournamentsInProgress.length > 0}
        <h2>In Progress</h2>
        <div class="list">
        {#each tournamentsInProgress as tournament}
            <TournamentCard tournament={tournament} />
        {/each}
        </div>
    {/if}

    {#if tournamentsUpcoming.length > 0}
        <h2>Upcoming</h2>
        <div class="list">
        {#each tournamentsUpcoming as tournament}
            <TournamentCard tournament={tournament} />
        {/each}
        </div>
    {/if}
</div>
</div>